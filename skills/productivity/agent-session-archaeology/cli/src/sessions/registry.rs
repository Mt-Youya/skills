use super::environment::Environment;
use crate::storage::files;
use anyhow::{Context, Result};
use serde_json::Value;
use std::{collections::BTreeSet, fs};

pub(super) fn load(environment: &Environment, project: &str) -> Result<BTreeSet<String>> {
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

pub(super) fn contains(ids: &BTreeSet<String>, id: &str) -> bool {
    ids.iter()
        .any(|wanted| wanted.trim_start_matches("session-") == id.trim_start_matches("session-"))
}
