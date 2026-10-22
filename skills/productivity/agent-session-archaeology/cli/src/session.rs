use crate::{files, redact};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: String,
    pub time: Option<i64>,
    pub text: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub source: String,
    pub id: String,
    pub file: PathBuf,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub messages: Vec<Message>,
    pub note: Option<String>,
    pub r#match: String,
    pub original_lines: usize,
    pub parse_errors: Vec<usize>,
    pub read_to_eof: bool,
}

impl Session {
    pub fn new(source: &str, id: String, file: PathBuf) -> Self {
        Self {
            source: source.into(),
            id,
            file,
            cwd: None,
            title: None,
            started_at: None,
            ended_at: None,
            messages: Vec::new(),
            note: None,
            r#match: String::new(),
            original_lines: 0,
            parse_errors: Vec::new(),
            read_to_eof: true,
        }
    }
    pub fn stamp(&mut self, time: Option<i64>) {
        if let Some(t) = time {
            self.started_at = Some(self.started_at.map_or(t, |old| old.min(t)));
            self.ended_at = Some(self.ended_at.map_or(t, |old| old.max(t)));
        }
    }
    pub fn message(&mut self, role: &str, text: String, time: Option<i64>, line: usize) {
        if !text.is_empty() {
            self.messages.push(Message {
                role: role.into(),
                text,
                time,
                line,
                tool: None,
                result: None,
            });
        }
    }
    pub fn tool(&mut self, name: &str, input: &Value, time: Option<i64>, line: usize) {
        let text = if input.is_null() {
            String::new()
        } else {
            input.to_string()
        };
        self.messages.push(Message {
            role: "tool".into(),
            text,
            time,
            line,
            tool: Some(name.into()),
            result: None,
        });
    }
    pub fn row(&self) -> Value {
        json!({"source":self.source,"id":self.id,"file":self.file,"cwd":self.cwd,"title":self.title.as_deref().map(redact::text),"startedAt":self.started_at,"endedAt":self.ended_at,"messages":self.messages.len(),"userMessages":self.messages.iter().filter(|m|m.role=="user").count(),"match":self.r#match,"bytes":fs::metadata(&self.file).map_or(0,|m|m.len()),"original_lines":self.original_lines,"parse_errors":self.parse_errors,"read_to_eof":self.read_to_eof,"semantic_review":"not_reviewed"})
    }
}

pub fn time(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|t| t.timestamp_millis())
    })
}

pub fn flatten(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.into();
    }
    value
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| match b["type"].as_str().unwrap_or("") {
                    "text" | "input_text" | "output_text" => b["text"].as_str().map(String::from),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string()
        })
        .unwrap_or_default()
}

pub fn tool_blocks(session: &mut Session, content: &Value, time: Option<i64>, line: usize) {
    if let Some(blocks) = content.as_array() {
        for block in blocks {
            if block["type"] == "tool_use" {
                session.tool(
                    block["name"].as_str().unwrap_or("unknown"),
                    &block["input"],
                    time,
                    line,
                );
            }
            if block["type"] == "tool_result" {
                session.message("tool", flatten(&block["content"]), time, line);
            }
        }
    }
}

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

pub fn timestamp(value: Option<i64>) -> String {
    value
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map_or_else(
            || "unknown".into(),
            |t| t.format("%Y-%m-%d %H:%M UTC").to_string(),
        )
}

pub fn markdown(session: &Session, full: bool) -> String {
    let mut output=format!("# {}\n\n- Source: {}\n- Session: {}\n- Working directory: {}\n- Time: {} → {}\n- Messages: {}\n- Original file: {}\n- Parse: EOF {}; original lines {}; errors {:?}\n- Semantic review: not_reviewed\n\n---\n\n",session.title.as_deref().unwrap_or(&session.id),session.source,session.id,session.cwd.as_deref().unwrap_or("(unknown)"),timestamp(session.started_at),timestamp(session.ended_at),session.messages.len(),session.file.display(),session.read_to_eof,session.original_lines,session.parse_errors);
    if let Some(note) = &session.note {
        output.push_str(&format!("Note: {note}\n\n"));
    }
    for message in &session.messages {
        if message.role == "tool" && !full {
            continue;
        }
        let who = match message.role.as_str() {
            "user" => "👤 User",
            "assistant" => "🤖 Assistant",
            "tool" => "🔧 Tool",
            _ => "⚙️ System",
        };
        output.push_str(&format!(
            "### {who} · {} · Original line {}\n\n",
            timestamp(message.time),
            message.line
        ));
        if let Some(tool) = &message.tool {
            output.push_str(&format!("Tool: {tool}\n\n"));
        }
        output.push_str(&message.text);
        output.push('\n');
        if let Some(result) = &message.result {
            output.push_str(result);
            output.push('\n');
        }
        output.push('\n');
    }
    redact::text(&output)
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
        if n >= from {
            records.push(json!({"line":n,"text":redact::text(text.trim_end_matches('\n'))}));
        }
    }
    if records.is_empty() {
        bail!("start line {from} exceeds file length {n}");
    }
    Ok(
        json!({"file":files::absolute(file)?,"from":from,"to":records.last().unwrap()["line"],"read_to_eof":eof,"next_from":if eof{Value::Null}else{json!(to.saturating_add(1))},"lines":records,"semantic_review":"not_reviewed"}),
    )
}
