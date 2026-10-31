use crate::storage::files;
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

// Metadata-only cache: no transcript bodies or inferred semantic-review state.
pub struct Inventory(BTreeMap<(String, PathBuf), Value>);

impl Inventory {
    pub fn load(path: Option<&Path>, project: &str) -> Result<Self> {
        let mut entries = BTreeMap::new();
        if let Some(path) = path {
            let value: Value = serde_json::from_slice(&fs::read(path)?)
                .context("cannot parse previous inventory")?;
            let data = value.get("data").unwrap_or(&value);
            anyhow::ensure!(
                data["project"] == project,
                "previous inventory belongs to a different project"
            );
            anyhow::ensure!(
                data["content_search"] != true,
                "content-search inventories cannot be reused for ownership-only scans"
            );
            let sessions = data["sessions"]
                .as_array()
                .context("previous inventory needs a sessions array")?;
            for row in sessions
                .iter()
                .chain(data["unchanged"].as_array().into_iter().flatten())
                .chain(data["not_seen"].as_array().into_iter().flatten())
            {
                if let (Some(source), Some(file)) = (row["source"].as_str(), row["file"].as_str()) {
                    entries.insert(
                        (source.into(), files::absolute(Path::new(file))?),
                        row.clone(),
                    );
                }
            }
        }
        Ok(Self(entries))
    }

    pub fn unchanged(&self, source: &str, file: &Path) -> Option<Value> {
        let key = (source.to_string(), files::absolute(file).ok()?);
        let row = self.0.get(&key)?;
        if row["read_to_eof"] != true || !row["parse_errors"].as_array()?.is_empty() {
            return None;
        }
        if row["file_state"] != state(file).ok()? {
            return None;
        }
        let mut row = row.clone();
        row["semantic_review"] = json!("not_reviewed");
        Some(row)
    }

    pub fn not_seen<'a>(&self, inputs: impl Iterator<Item = (&'a str, &'a Path)>) -> Vec<Value> {
        let observed: std::collections::BTreeSet<_> = inputs
            .filter_map(|(source, file)| Some((source.to_string(), files::absolute(file).ok()?)))
            .collect();
        self.0
            .iter()
            .filter(|(key, _)| !observed.contains(*key))
            .map(|(_, row)| row.clone())
            .collect()
    }
}

pub fn state(file: &Path) -> Result<Value> {
    let metadata = fs::metadata(file)?;
    let modified = metadata.modified()?.duration_since(UNIX_EPOCH)?;
    let mut value = json!({"bytes":metadata.len(), "modified_ns":modified.as_nanos().to_string()});
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        value["device"] = json!(metadata.dev().to_string());
        value["inode"] = json!(metadata.ino().to_string());
        value["changed_seconds"] = json!(metadata.ctime().to_string());
        value["changed_ns"] = json!(metadata.ctime_nsec().to_string());
    }
    Ok(value)
}
