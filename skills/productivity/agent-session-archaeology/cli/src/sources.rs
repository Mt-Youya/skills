use crate::{
    adapters,
    environment::Environment,
    files,
    session::{self, Session},
};
use anyhow::{Context, Result};
use percent_encoding::percent_decode_str;
use regex::Regex;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::BufRead,
    path::{Path, PathBuf},
};

pub struct Discovery {
    pub sessions: Vec<Session>,
    pub diagnostics: Vec<Value>,
    pub registry: Value,
    pub project: PathBuf,
}

impl Discovery {
    fn incomplete(&self) -> bool {
        self.diagnostics.iter().any(|d| d["reason"].is_string())
            || self.sessions.iter().any(|s| !s.parse_errors.is_empty())
            || self.registry["missing"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
    }
    pub fn report(&self) -> Value {
        json!({"project":files::norm(&self.project.to_string_lossy()),"root":self.project,"sessions":self.sessions.iter().map(Session::row).collect::<Vec<_>>(),"diagnostics":self.diagnostics,"registry":self.registry,"incomplete":self.incomplete(),"semantic_review":"not_reviewed"})
    }
}

fn existing_files(root: &Path, diagnostics: &mut Vec<Value>) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    match files::walk(root, 8) {
        Ok(files) => files,
        Err(error) => {
            diagnostics.push(json!({"path":root,"reason":error.to_string()}));
            Vec::new()
        }
    }
}

fn registry(environment: &Environment, project: &str) -> Result<BTreeSet<String>> {
    let file = environment.home.join(".dsh/storages/workspace.json");
    let bytes = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeSet::new()),
        Err(e) => return Err(e).context("cannot read DSH registry"),
    };
    let value: Value = serde_json::from_slice(&bytes).context("cannot parse DSH registry")?;
    let mut ids = BTreeSet::new();
    if let Some(workspaces) = value["tables"]["workspaces"].as_object() {
        for ws in workspaces.values() {
            if ws["path"]
                .as_str()
                .is_some_and(|path| files::norm(path) == project)
            {
                if let Some(list) = ws["sessionIds"].as_array() {
                    for id in list {
                        if let Some(id) = id.as_str() {
                            ids.insert(id.into());
                        }
                    }
                }
            }
        }
    }
    Ok(ids)
}

fn registered(ids: &BTreeSet<String>, id: &str) -> bool {
    ids.iter()
        .any(|wanted| wanted.trim_start_matches("session-") == id.trim_start_matches("session-"))
}

fn parse_file(
    source: &str,
    file: &Path,
    project: &str,
    content: bool,
    registered_ids: &BTreeSet<String>,
    folder: Option<&str>,
) -> Result<Option<Session>> {
    let fallback = file
        .file_stem()
        .context("session file lacks filename")?
        .to_string_lossy()
        .to_string();
    let fallback = if source == "dsh" {
        file.parent()
            .and_then(|p| p.file_name())
            .map(|v| v.to_string_lossy().into_owned())
            .unwrap_or(fallback)
    } else {
        fallback
    };
    let mut session = Session::new(source, fallback, file.into());
    session.cwd = folder.map(String::from);
    if folder.is_some_and(|cwd| !files::within(cwd, project)) && !content {
        return Ok(None);
    }
    let mut rows = Vec::new();
    let mut reader = session::input(file)?;
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        session.original_lines += 1;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                session.parse_errors.push(session.original_lines);
                continue;
            }
        };
        let meta = if value["type"] == "session_meta" {
            &value["payload"]
        } else {
            &value
        };
        if (value["type"] == "session_meta" || value["type"] == "session") && meta["id"].is_string()
        {
            session.id = meta["id"].as_str().unwrap().into();
        }
        if session.cwd.is_none() {
            session.cwd = meta["cwd"].as_str().map(String::from);
        }
        let in_registry = source == "dsh" && registered(registered_ids, &session.id);
        if !content
            && !in_registry
            && session
                .cwd
                .as_ref()
                .is_some_and(|cwd| !files::within(cwd, project))
        {
            return Ok(None);
        }
        rows.push((session.original_lines, value));
    }
    if source == "copilot" {
        adapters::copilot(&mut session, &rows)?;
    } else {
        for (line, value) in rows {
            adapters::row(&mut session, &value, line)?;
        }
    }
    session.r#match = if session
        .cwd
        .as_ref()
        .is_some_and(|cwd| files::within(cwd, project))
    {
        "cwd"
    } else if source == "dsh" && registered(registered_ids, &session.id) {
        "registry"
    } else if content
        && session.messages.iter().any(|m| {
            let text = m.text.to_lowercase();
            text.contains(project)
                || text.contains(&project.replace('/', "\\"))
                || (cfg!(target_os = "macos")
                    && project
                        .strip_prefix("/private/")
                        .is_some_and(|p| text.contains(&format!("/{p}"))))
        })
    {
        "content"
    } else {
        if !session.parse_errors.is_empty() {
            anyhow::bail!(
                "unmatched session has invalid JSON at lines {:?}",
                session.parse_errors
            );
        }
        return Ok(None);
    }
    .into();
    if session.messages.is_empty() {
        if !session.parse_errors.is_empty() {
            anyhow::bail!(
                "session has no readable messages; invalid JSON at lines {:?}",
                session.parse_errors
            );
        }
        return Ok(None);
    }
    Ok(Some(session))
}

pub fn discover(environment: &Environment, project: &Path, content: bool) -> Result<Discovery> {
    let project = files::absolute(project)?;
    let normalized = files::norm(&project.to_string_lossy());
    let mut diagnostics = Vec::new();
    let ids = match registry(environment, &normalized) {
        Ok(ids) => ids,
        Err(e) => {
            diagnostics.push(json!({"source":"dsh-registry","reason":e.to_string()}));
            BTreeSet::new()
        }
    };
    let mut sessions = Vec::new();
    let mut inputs = Vec::<(String, PathBuf, Option<String>)>::new();
    for (source, root) in [
        ("claude-code", environment.home.join(".claude/projects")),
        ("codex", environment.home.join(".codex/sessions")),
    ] {
        let files: Vec<_> = existing_files(&root, &mut diagnostics)
            .into_iter()
            .filter(|path| {
                let name = path.file_name().unwrap().to_string_lossy();
                name.ends_with(".jsonl")
                    || (source == "codex"
                        && (name.ends_with(".jsonl.zst") || name.ends_with(".jsonl.zstd"))
                        && !path.with_extension("").exists())
            })
            .collect();
        diagnostics
            .push(json!({"source":source,"available":root.exists(),"candidate_files":files.len()}));
        for path in files {
            inputs.push((source.into(), path, None));
        }
    }
    let dsh = environment.home.join(".dsh/sessions");
    let mut versions = BTreeMap::<PathBuf, (u64, PathBuf)>::new();
    let version_re = Regex::new(r"^session\.v(\d+)\.jsonl\.zstd$")?;
    for path in existing_files(&dsh, &mut diagnostics) {
        if let Some(caps) = path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| version_re.captures(s))
        {
            let version = caps[1].parse::<u64>()?;
            let parent = path.parent().unwrap().to_path_buf();
            if versions.get(&parent).is_none_or(|(old, _)| version > *old) {
                versions.insert(parent, (version, path));
            }
        }
    }
    diagnostics
        .push(json!({"source":"dsh","available":dsh.exists(),"candidate_files":versions.len()}));
    for (_, (_, path)) in versions {
        inputs.push(("dsh".into(), path, None));
    }
    let mut seen = BTreeSet::new();
    for store in environment.vscode_stores() {
        if !store.exists() || !seen.insert(store.clone()) {
            continue;
        }
        for file in existing_files(&store, &mut diagnostics)
            .into_iter()
            .filter(|p| p.file_name().is_some_and(|s| s == "workspace.json"))
        {
            let read_folder = || -> Result<String> {
                let value: Value = serde_json::from_slice(&fs::read(&file)?)?;
                let folder = value["folder"]
                    .as_str()
                    .context("workspace folder is missing")?;
                Ok(if let Some(uri) = folder.strip_prefix("file://") {
                    percent_decode_str(uri).decode_utf8()?.into_owned()
                } else {
                    folder.into()
                })
            };
            let folder = match read_folder() {
                Ok(folder) => folder,
                Err(e) => {
                    diagnostics
                        .push(json!({"source":"copilot","path":file,"reason":e.to_string()}));
                    continue;
                }
            };
            if !content && !files::within(&folder, &normalized) {
                continue;
            }
            for path in existing_files(
                &file.parent().unwrap().join("chatSessions"),
                &mut diagnostics,
            ) {
                if path.extension().is_some_and(|e| e == "jsonl") {
                    inputs.push(("copilot".into(), path, Some(folder.clone())));
                }
            }
        }
    }
    for (source, file, folder) in inputs {
        match parse_file(
            &source,
            &file,
            &normalized,
            content,
            &ids,
            folder.as_deref(),
        ) {
            Ok(Some(session)) => sessions.push(session),
            Ok(None) => {}
            Err(error) => {
                diagnostics.push(json!({"source":source,"file":file,"reason":error.to_string()}))
            }
        }
    }
    sessions.sort_by_key(|s| s.started_at.unwrap_or(0));
    let mut identities = BTreeSet::new();
    sessions.retain(|s|{
        if identities.insert((s.source.clone(),s.id.clone())){true}else{
            diagnostics.push(json!({"source":s.source,"file":s.file,"reason":"duplicate source/session ID; retained first file"}));false
        }
    });
    let found: BTreeSet<_> = sessions
        .iter()
        .filter(|s| s.source == "dsh")
        .map(|s| s.id.clone())
        .collect();
    let missing: Vec<_> = ids.iter().filter(|id| !registered(&found, id)).collect();
    let registry = json!({"dsh_expected":ids.len(),"dsh_found":found.len(),"missing":missing});
    Ok(Discovery {
        sessions,
        diagnostics,
        registry,
        project,
    })
}

pub fn export(discovery: &Discovery, out: &Path, full: bool, dry_run: bool) -> Result<Value> {
    let out = files::absolute(out)?;
    let safe = Regex::new(r"[^a-zA-Z0-9._-]")?;
    let mut outputs = Vec::new();
    let mut index=String::from("# Agent sessions for this project\n\n| Source | Session | Messages | Export |\n| --- | --- | --- | --- |\n");
    for session in &discovery.sessions {
        let name = format!(
            "{}--{}.md",
            session.source,
            safe.replace_all(&session.id, "_")
        );
        let path = out.join(&name);
        let text = session::markdown(session, full);
        let changed = if dry_run {
            fs::read(&path).map_or(true, |old| old != text.as_bytes())
        } else {
            files::atomic_write(&path, text.as_bytes())?
        };
        outputs.push(json!({"path":path,"bytes":text.len(),"changed":changed,"source":session.source,"id":session.id}));
        index.push_str(&format!(
            "| {} | `{}` | {} | [{}](./{}) |\n",
            session.source,
            session.id,
            session.messages.len(),
            name,
            name
        ));
    }
    if !dry_run && !outputs.is_empty() {
        files::atomic_write(
            &out.join("index.md"),
            crate::redact::text(&index).as_bytes(),
        )?;
        files::atomic_write(
            &out.join("coverage.json"),
            &serde_json::to_vec_pretty(&discovery.report())?,
        )?;
        crate::redact::directory(&out, false)?;
    }
    Ok(
        json!({"out":out,"files":outputs,"dry_run":dry_run,"diagnostics":discovery.diagnostics,"registry":discovery.registry,"incomplete":discovery.incomplete(),"semantic_review":"not_reviewed"}),
    )
}
