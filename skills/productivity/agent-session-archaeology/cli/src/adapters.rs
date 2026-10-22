use crate::session::{self, Session};
use anyhow::{bail, Result};
use serde_json::{json, Value};

pub fn row(session: &mut Session, value: &Value, line: usize) -> Result<()> {
    let time = session::time(&value["timestamp"])
        .or_else(|| session::time(&value["time"]))
        .or_else(|| session::time(&value["data"]["time"]));
    session.stamp(time);
    match session.source.as_str() {
        "claude-code" => claude(session, value, line, time),
        "codex" => codex(session, value, line, time),
        "dsh" => dsh(session, value, line, time),
        _ => {}
    }
    Ok(())
}

fn claude(session: &mut Session, value: &Value, line: usize, time: Option<i64>) {
    if value["type"] == "ai-title" && session.title.is_none() {
        session.title = value["title"].as_str().map(String::from);
    }
    if let Some(role @ ("user" | "assistant")) = value["type"].as_str() {
        let content = &value["message"]["content"];
        session::tool_blocks(session, content, time, line);
        session.message(role, session::flatten(content), time, line);
    }
}

fn codex(session: &mut Session, value: &Value, line: usize, time: Option<i64>) {
    if value["type"] == "session_meta" {
        session.stamp(session::time(&value["payload"]["timestamp"]));
    }
    let p = &value["payload"];
    if value["type"] == "response_item" {
        match p["type"].as_str().unwrap_or("") {
            "message" => {
                if let Some(role @ ("user" | "assistant")) = p["role"].as_str() {
                    session.message(role, session::flatten(&p["content"]), time, line);
                }
            }
            "function_call" | "custom_tool_call" => session.tool(
                p["name"].as_str().unwrap_or("unknown"),
                p.get("arguments")
                    .or_else(|| p.get("input"))
                    .unwrap_or(&Value::Null),
                time,
                line,
            ),
            "function_call_output" | "custom_tool_call_output" => session.message(
                "tool",
                p["output"]
                    .as_str()
                    .map(String::from)
                    .unwrap_or_else(|| p["output"].to_string()),
                time,
                line,
            ),
            _ => {}
        }
    } else if value["type"] == "event_msg" && p["type"] == "item_completed" {
        let item = &p["item"];
        let kind = item["type"].as_str().unwrap_or("").to_lowercase();
        let role = if kind.contains("usermessage") {
            "user"
        } else if kind.contains("agentmessage") || kind.contains("assistantmessage") {
            "assistant"
        } else {
            return;
        };
        let text = item["content"]
            .as_array()
            .map(|v| {
                v.iter()
                    .filter_map(|v| v["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        session.message(role, text, time, line);
    }
}

fn dsh(session: &mut Session, value: &Value, line: usize, time: Option<i64>) {
    let data = &value["data"];
    match value["type"].as_str().unwrap_or("") {
        "session" => session.stamp(session::time(&value["createdAt"])),
        "user/message" => session.message("user", session::flatten(&data["content"]), time, line),
        "assistant/message" => {
            session::tool_blocks(session, &data["message"]["content"], time, line);
            session.message(
                "assistant",
                session::flatten(&data["message"]["content"]),
                time,
                line,
            );
        }
        "tool/call" => session.tool(
            data["name"].as_str().unwrap_or("unknown"),
            data.get("arguments")
                .or_else(|| data.get("input"))
                .unwrap_or(&Value::Null),
            time,
            line,
        ),
        "tool/result" => session.message(
            "tool",
            session::flatten(&data["message"]["content"]),
            time,
            line,
        ),
        "session/title" => {
            if session.title.is_none() {
                session.title = data["title"].as_str().map(String::from);
            }
        }
        "compaction/summary" => {
            if let Some(summary) = data["summary"].as_str() {
                session.message("system", format!("[compaction] {summary}"), time, line);
            }
        }
        _ => {}
    }
}

pub fn patch(target: &mut Value, keys: &[Value], value: Value) -> Result<()> {
    if keys.is_empty() {
        bail!("empty workspace patch path");
    }
    if let Some(index) = keys[0].as_u64() {
        let index = usize::try_from(index)?;
        if !target.is_array() {
            *target = json!([]);
        }
        let items = target.as_array_mut().unwrap();
        if index > items.len() {
            bail!("workspace patch skips array indexes");
        }
        if index == items.len() {
            items.push(Value::Null);
        }
        if keys.len() == 1 {
            items[index] = value;
        } else {
            patch(&mut items[index], &keys[1..], value)?;
        }
    } else if let Some(key) = keys[0].as_str() {
        if !target.is_object() {
            *target = json!({});
        }
        if keys.len() == 1 {
            target[key] = value;
        } else {
            patch(&mut target[key], &keys[1..], value)?;
        }
    } else {
        bail!("workspace patch key must be string or array index");
    }
    Ok(())
}

pub fn copilot(session: &mut Session, rows: &[(usize, Value)]) -> Result<()> {
    let mut state = Value::Null;
    for (_, row) in rows {
        if row["kind"] == 0 {
            state = row["v"].clone();
        } else if row["kind"] == 1 && !state.is_null() {
            let keys = row["k"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("workspace patch needs array path"))?;
            patch(&mut state, keys, row["v"].clone())?;
        }
    }
    session.stamp(session::time(&state["creationDate"]));
    if let Some(requests) = state["requests"].as_array() {
        for req in requests {
            let time = session::time(&req["timestamp"]);
            session.stamp(time);
            // A reconstructed request has no single original line; zero denotes patch-derived data.
            session.message(
                "user",
                req["message"]["text"].as_str().unwrap_or("").into(),
                time,
                0,
            );
            if let Some(parts) = req["response"].as_array() {
                for part in parts {
                    session.message(
                        "assistant",
                        part["value"].as_str().unwrap_or("").into(),
                        time,
                        0,
                    );
                }
            }
        }
    }
    session.note=Some("Reconstructed workspace patch state; original line 0 means patch-derived, not a transcript coordinate".into());
    Ok(())
}
