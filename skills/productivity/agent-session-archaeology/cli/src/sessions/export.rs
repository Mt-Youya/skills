use super::{discovery::Discovery, model as session};
use crate::storage::{files, redact};
use anyhow::Result;
use regex::Regex;
use serde_json::{json, Value};
use std::{fs, path::Path};

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
        files::atomic_write(&out.join("index.md"), redact::text(&index).as_bytes())?;
        files::atomic_write(
            &out.join("coverage.json"),
            &serde_json::to_vec_pretty(&discovery.report())?,
        )?;
        redact::directory(&out, false)?;
    }
    Ok(
        json!({"out":out,"files":outputs,"dry_run":dry_run,"diagnostics":discovery.diagnostics,"registry":discovery.registry,"incomplete":discovery.incomplete(),"semantic_review":"not_reviewed"}),
    )
}
