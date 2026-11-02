use crate::storage::{files, redact};
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

pub fn input(file: &Path) -> Result<Box<dyn BufRead>> {
    let input = fs::File::open(file).with_context(|| format!("cannot open {}", file.display()))?;
    if matches!(
        file.extension().and_then(|v| v.to_str()),
        Some("zstd" | "zst")
    ) {
        Ok(Box::new(BufReader::new(zstd::stream::read::Decoder::new(
            input,
        )?)))
    } else {
        Ok(Box::new(BufReader::new(input)))
    }
}

pub fn page(file: &Path, from: usize, to: Option<usize>, limit: usize) -> Result<Value> {
    if from == 0 || limit == 0 {
        bail!("--from and --limit must be greater than zero");
    }
    let to = to.unwrap_or_else(|| from.saturating_add(limit - 1));
    if to < from {
        bail!("--to precedes --from");
    }
    let mut reader = input(file)?;
    let mut text = String::new();
    let mut records = Vec::new();
    let mut n = 0;
    let mut eof = false;
    let mut redactor = redact::LineRedactor::default();
    loop {
        text.clear();
        if reader.read_line(&mut text)? == 0 {
            eof = true;
            break;
        }
        n += 1;
        if n > to {
            break;
        }
        let redacted = redactor.text(text.trim_end_matches('\n'));
        if n >= from {
            records.push(json!({"line":n,"text":redacted}));
        }
    }
    if records.is_empty() {
        bail!("start line {from} exceeds file length {n}");
    }
    Ok(
        json!({"file":files::absolute(file)?,"from":from,"to":records.last().unwrap()["line"],"read_to_eof":eof,"next_from":if eof{Value::Null}else{json!(to.saturating_add(1))},"lines":records,"semantic_review":"not_reviewed"}),
    )
}
