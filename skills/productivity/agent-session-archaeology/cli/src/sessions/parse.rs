use super::registry::contains as registered;
use super::{adapters, incremental, model::Session, read};
use crate::storage::files;
use anyhow::{Context, Result};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

pub(super) fn parse_file(
    source: &str,
    file: &Path,
    project: &str,
    content: bool,
    registered_ids: &BTreeSet<String>,
    folder: Option<&str>,
    wanted_id: Option<&str>,
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
    let before = incremental::state(file)?;
    session.cwd = folder.map(String::from);
    // cwd is assigned only once. Avoid filesystem canonicalization for every event.
    let mut cwd_matches = session.cwd.as_ref().map(|cwd| files::within(cwd, project));
    if cwd_matches == Some(false) && !content {
        return Ok(None);
    }
    let mut rows = Vec::new();
    let mut reader = read::input(file)?;
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
            // Codex/DSH headers identify one session, independently of its filename.
            // A targeted resolve need not decode unrelated conversation bodies.
            if matches!(source, "codex" | "dsh")
                && wanted_id.is_some_and(|wanted| wanted != session.id)
            {
                return Ok(None);
            }
        }
        if session.cwd.is_none() {
            session.cwd = meta["cwd"].as_str().map(String::from);
            cwd_matches = session.cwd.as_ref().map(|cwd| files::within(cwd, project));
        }
        let in_registry = source == "dsh" && registered(registered_ids, &session.id);
        if !content && !in_registry && cwd_matches == Some(false) {
            return Ok(None);
        }
        if source == "copilot" {
            rows.push((session.original_lines, value));
        } else {
            let line = session.original_lines;
            adapters::row(&mut session, &value, line)?;
        }
    }
    if source == "copilot" {
        adapters::copilot(&mut session, &rows)?;
    }
    let after = incremental::state(file)?;
    if before == after {
        session.file_state = Some(after);
    } else {
        session.read_to_eof = false;
        session.note =
            Some("Original changed during parsing; rescan before reusing its inventory".into());
    }
    if wanted_id.is_some_and(|wanted| wanted != session.id) {
        return Ok(None);
    }
    session.r#match = if cwd_matches == Some(true) {
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
