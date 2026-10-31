use super::registry::{contains as registered, load as registry};
use super::{environment::Environment, incremental::Inventory, model::Session, parse::parse_file};
use crate::storage::files;
use anyhow::{Context, Result};
use percent_encoding::percent_decode_str;
use regex::Regex;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

pub struct Discovery {
    pub sessions: Vec<Session>,
    pub diagnostics: Vec<Value>,
    pub registry: Value,
    pub project: PathBuf,
    pub unchanged: Vec<Value>,
    pub not_seen: Vec<Value>,
    pub incremental: bool,
    pub content_search: bool,
}

impl Discovery {
    pub(super) fn incomplete(&self) -> bool {
        self.diagnostics.iter().any(|d| d["reason"].is_string())
            || self
                .sessions
                .iter()
                .any(|s| !s.parse_errors.is_empty() || !s.read_to_eof)
            || self.registry["missing"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
    }
    pub fn report(&self) -> Value {
        json!({"project":files::norm(&self.project.to_string_lossy()),"root":self.project,"sessions":self.sessions.iter().map(Session::row).collect::<Vec<_>>(),"unchanged":self.unchanged,"not_seen":self.not_seen,"incremental":self.incremental,"content_search":self.content_search,"diagnostics":self.diagnostics,"registry":self.registry,"incomplete":self.incomplete(),"semantic_review":"not_reviewed"})
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

pub fn discover(environment: &Environment, project: &Path, content: bool) -> Result<Discovery> {
    discover_filtered(environment, project, content, None, None)
}

pub fn scan(
    environment: &Environment,
    project: &Path,
    content: bool,
    since: Option<&Path>,
) -> Result<Discovery> {
    discover_filtered(environment, project, content, None, since)
}

pub fn resolve(
    environment: &Environment,
    project: &Path,
    content: bool,
    id: &str,
    source: Option<&str>,
) -> Result<Value> {
    let discovery = discover_filtered(environment, project, content, Some((id, source)), None)?;
    let found: Vec<_> = discovery.sessions.iter().collect();
    if found.len() != 1 {
        anyhow::bail!(
            "expected one source/session match for {id}, found {}; use --source to disambiguate",
            found.len()
        );
    }
    Ok(found[0].row())
}

fn discover_filtered(
    environment: &Environment,
    project: &Path,
    content: bool,
    target: Option<(&str, Option<&str>)>,
    since: Option<&Path>,
) -> Result<Discovery> {
    let includes =
        |source: &str| target.is_none_or(|(_, wanted)| wanted.is_none_or(|s| s == source));
    let project = files::absolute(project)?;
    let normalized = files::norm(&project.to_string_lossy());
    let previous = Inventory::load(since, &normalized)?;
    let mut unchanged = Vec::new();
    let mut diagnostics = Vec::new();
    let ids = match if includes("dsh") {
        registry(environment, &normalized)
    } else {
        Ok(BTreeSet::new())
    } {
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
        if !includes(source) {
            continue;
        }
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
    for path in if includes("dsh") {
        existing_files(&dsh, &mut diagnostics)
    } else {
        Vec::new()
    } {
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
        if !includes("copilot") || !store.exists() || !seen.insert(store.clone()) {
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
        if let Some(row) = previous.unchanged(&source, &file) {
            let owned = row["cwd"]
                .as_str()
                .is_some_and(|cwd| files::within(cwd, &normalized))
                || (source == "dsh" && row["id"].as_str().is_some_and(|id| registered(&ids, id)));
            if owned {
                unchanged.push(row);
                continue;
            }
        }
        match parse_file(
            &source,
            &file,
            &normalized,
            content,
            &ids,
            folder.as_deref(),
            target.map(|(id, _)| id),
        ) {
            Ok(Some(session)) => sessions.push(session),
            Ok(None) => {}
            Err(error) => {
                diagnostics.push(json!({"source":source,"file":file,"reason":error.to_string()}))
            }
        }
    }
    sessions.sort_by_key(|s| s.started_at.unwrap_or(0));
    let mut identities: BTreeSet<_> = unchanged
        .iter()
        .filter_map(|row| {
            Some((
                row["source"].as_str()?.to_string(),
                row["id"].as_str()?.to_string(),
            ))
        })
        .collect();
    sessions.retain(|s|{
        if target.is_some() || identities.insert((s.source.clone(),s.id.clone())){true}else{
            diagnostics.push(json!({"source":s.source,"file":s.file,"reason":"duplicate source/session ID; retained first file"}));false
        }
    });
    let not_seen = previous.not_seen(
        sessions
            .iter()
            .map(|s| (s.source.as_str(), s.file.as_path()))
            .chain(unchanged.iter().filter_map(|row| {
                Some((row["source"].as_str()?, Path::new(row["file"].as_str()?)))
            })),
    );
    let found: BTreeSet<_> = sessions
        .iter()
        .filter(|s| s.source == "dsh")
        .map(|s| s.id.clone())
        .chain(
            unchanged
                .iter()
                .filter(|row| row["source"] == "dsh")
                .filter_map(|row| row["id"].as_str().map(String::from)),
        )
        .collect();
    let missing: Vec<_> = ids.iter().filter(|id| !registered(&found, id)).collect();
    let registry = json!({"dsh_expected":ids.len(),"dsh_found":found.len(),"missing":missing});
    Ok(Discovery {
        sessions,
        diagnostics,
        registry,
        project,
        unchanged,
        not_seen,
        incremental: since.is_some(),
        content_search: content,
    })
}
