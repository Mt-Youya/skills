use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Fixture {
    root: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("stores");
        let project = root.path().join("project");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&project).unwrap();
        Self {
            root,
            home,
            project,
        }
    }
    fn file(&self, path: &str, text: &str) -> PathBuf {
        let file = self.home.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, text).unwrap();
        file
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sarch"))
            .current_dir(&self.project)
            .env("SESSION_ARCHAEOLOGY_HOME", &self.home)
            .arg("--json")
            .args(args)
            .output()
            .unwrap()
    }
    fn scan(&self) -> Value {
        let result = self.run(&["sessions", "scan"]);
        let report = serde_json::from_slice::<Value>(&result.stdout).unwrap();
        assert!(report["data"].is_object(), "{report}");
        assert_eq!(
            result.status.success(),
            report["data"]["incomplete"] != true
        );
        report["data"].clone()
    }
    fn codex(&self, id: &str, cwd: &Path) -> PathBuf {
        self.file(&format!(".codex/sessions/2026/10/22/{id}.jsonl"),&format!("{}\n{}\n{}\n{}\n",json!({"type":"session_meta","payload":{"id":id,"cwd":cwd,"timestamp":"2026-10-22T01:00:00Z"}}),json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"find failure"}]}}),json!({"type":"response_item","payload":{"type":"function_call","name":"terminal","arguments":"SESSDATA=abcdefghijklmnop"}}),json!({"type":"response_item","payload":{"type":"function_call_output","output":"actual tool result"}})))
    }
}

#[test]
fn help_and_offline_doctor_work_without_accounts_or_sources() {
    let f = Fixture::new();
    let help = Command::new(env!("CARGO_BIN_EXE_sarch"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("evidence"));
    let result = f.run(&["doctor"]);
    assert!(result.status.success());
    let data: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(data["data"]["auth"]["required"], false);
    assert_eq!(data["data"]["mode"], "offline");
    assert_eq!(data["data"]["isolated_stores"], true);
    assert_eq!(data["data"]["data_found"], false);
    let error = f.run(&["evidence", "not-a-command"]);
    assert!(!error.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&error.stdout).unwrap()["ok"],
        false
    );
}

#[test]
fn codex_subdirectories_resolve_and_private_exports_are_incremental() {
    let f = Fixture::new();
    f.codex("owned", &f.project.join("subdir"));
    let data = f.scan();
    assert_eq!(data["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(data["sessions"][0]["match"], "cwd");
    assert_eq!(data["sessions"][0]["semantic_review"], "not_reviewed");
    assert!(f
        .run(&["sessions", "resolve", "--id", "owned", "--source", "codex"])
        .status
        .success());
    assert!(f
        .run(&["sessions", "export", "--full", "--dry-run"])
        .status
        .success());
    assert!(!f.project.join(".agent-sessions").exists());
    let first = f.run(&["sessions", "export", "--full"]);
    assert!(first.status.success());
    let export = f.project.join(".agent-sessions/codex--owned.md");
    let text = fs::read_to_string(&export).unwrap();
    assert!(text.contains("actual tool result"));
    assert!(text.contains("Original line 4"));
    assert!(text.contains("SESSDATA=[redacted]"));
    assert!(!text.contains("abcdefghijklmnop"));
    assert!(f.run(&["redact", "--check"]).status.success());
    let second = f.run(&["sessions", "export", "--full"]);
    let data: Value = serde_json::from_slice(&second.stdout).unwrap();
    assert_eq!(data["data"]["files"][0]["changed"], false);
}

#[test]
fn external_ownership_skips_body_and_content_fallback_is_explicit() {
    let f = Fixture::new();
    let external = f.root.path().join("other");
    f.file(
        ".codex/sessions/external.jsonl",
        &format!(
            "{}\nnot JSON or even UTF8\n",
            json!({"type":"session_meta","payload":{"id":"outside","cwd":external}})
        ),
    );
    let data = f.scan();
    assert!(data["sessions"].as_array().unwrap().is_empty());
    assert!(data["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .all(|d| d["reason"].is_null()));
    let path=f.file(".claude/projects/shared/session.jsonl",&format!("{}\n",json!({"cwd":external,"type":"user","message":{"content":format!("cross repo {}",f.project.display())}})));
    assert!(path.exists());
    let result = f.run(&["sessions", "scan", "--content"]);
    let data: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(data["data"]["sessions"][0]["match"], "content");
}

#[test]
fn claude_tools_results_and_parse_gaps_are_retained() {
    let f = Fixture::new();
    f.file(".claude/projects/project/session.jsonl",&format!("{}\n{}\n{{broken tail\n",json!({"cwd":f.project,"type":"assistant","message":{"content":[{"type":"text","text":"investigate"},{"type":"tool_use","name":"shell","input":{"cmd":"pwd"}}]}}),json!({"cwd":f.project,"type":"user","message":{"content":[{"type":"tool_result","content":"tool failure"}]}})));
    let data = f.scan();
    assert_eq!(data["sessions"][0]["parse_errors"], json!([3]));
    assert_eq!(
        f.run(&["sessions", "export", "--full"]).status.code(),
        Some(1)
    );
    let text =
        fs::read_to_string(f.project.join(".agent-sessions/claude-code--session.md")).unwrap();
    assert!(text.contains("pwd"));
    assert!(text.contains("tool failure"));
}

#[test]
fn dsh_multi_frame_highest_version_and_registry_are_reconciled() {
    let f = Fixture::new();
    f.file(".dsh/storages/workspace.json",&json!({"tables":{"workspaces":{"one":{"path":f.project,"sessionIds":["session-dsh-id","missing-id"]}}}}).to_string());
    let header = format!(
        "{}\n",
        json!({"type":"session","id":"dsh-id","cwd":f.root.path().join("elsewhere"),"createdAt":"2026-10-22T02:00:00Z"})
    );
    let body = format!(
        "{}\n{}\n",
        json!({"type":"user/message","data":{"content":"question"}}),
        json!({"type":"assistant/message","data":{"message":{"content":[{"type":"text","text":"tail frame answer"}]}}})
    );
    let file = f.file(
        ".dsh/sessions/irreversible/session-dsh-id/session.v4.jsonl.zstd",
        "",
    );
    let mut packed = zstd::stream::encode_all(header.as_bytes(), 1).unwrap();
    packed.extend(zstd::stream::encode_all(body.as_bytes(), 1).unwrap());
    fs::write(&file, &packed).unwrap();
    fs::write(
        file.with_file_name("session.v3.jsonl.zstd"),
        "damaged old version",
    )
    .unwrap();
    let data = f.scan();
    assert_eq!(data["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(data["sessions"][0]["match"], "registry");
    assert_eq!(data["sessions"][0]["original_lines"], 3);
    assert_eq!(data["registry"]["missing"], json!(["missing-id"]));
    assert_eq!(f.run(&["sessions", "export"]).status.code(), Some(1));
    assert!(
        fs::read_to_string(f.project.join(".agent-sessions/dsh--dsh-id.md"))
            .unwrap()
            .contains("tail frame answer")
    );
    fs::write(file, &packed[..packed.len() - 5]).unwrap();
    let data = f.scan();
    assert!(data["sessions"].as_array().unwrap().is_empty());
    assert!(data["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["source"] == "dsh" && d["reason"].is_string()));
}

#[test]
fn workspace_patch_streams_are_reconstructed_and_empty_states_excluded() {
    let f = Fixture::new();
    f.file(
        ".config/Code/User/workspaceStorage/hash/workspace.json",
        &json!({"folder":format!("file://{}",f.project.display())}).to_string(),
    );
    f.file(
        ".config/Code/User/workspaceStorage/hash/chatSessions/empty.jsonl",
        &format!("{}\n", json!({"kind":0,"v":{"requests":[]}})),
    );
    f.file(".config/Code/User/workspaceStorage/hash/chatSessions/chat.jsonl",&format!("{}\n{}\n{}\n",json!({"kind":0,"v":{"requests":[]}}),json!({"kind":1,"k":["requests",0],"v":{"message":{"text":"user prompt"},"response":[]}}),json!({"kind":1,"k":["requests",0,"response",0],"v":{"value":"assistant reply"}})));
    let data = f.scan();
    assert_eq!(data["sessions"].as_array().unwrap().len(), 1);
    assert_eq!(data["sessions"][0]["source"], "copilot");
    assert_eq!(data["sessions"][0]["messages"], 2);
}

#[test]
fn raw_paging_is_bounded_and_redacted() {
    let f = Fixture::new();
    let file = f.file(
        "authorized.txt",
        "first\nSESSDATA=abcdefghijklmnop\nthird\nfourth\n",
    );
    let result = f.run(&[
        "read",
        "--file",
        file.to_str().unwrap(),
        "--from",
        "2",
        "--limit",
        "2",
    ]);
    assert!(result.status.success());
    let data: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(data["data"]["lines"].as_array().unwrap().len(), 2);
    assert_eq!(data["data"]["lines"][0]["line"], 2);
    assert_eq!(data["data"]["lines"][0]["text"], "SESSDATA=[redacted]");
    assert_eq!(data["data"]["next_from"], 4);
    assert_eq!(data["data"]["read_to_eof"], false);
    assert!(!f
        .run(&["read", "--file", file.to_str().unwrap(), "--from", "0"])
        .status
        .success());
}

#[test]
fn compressed_codex_siblings_are_discovered_when_plain_original_is_gone() {
    let f = Fixture::new();
    let plain = f.codex("compressed", &f.project);
    let compressed = plain.with_extension("jsonl.zst");
    fs::write(
        &compressed,
        zstd::stream::encode_all(fs::read(&plain).unwrap().as_slice(), 1).unwrap(),
    )
    .unwrap();
    assert_eq!(f.scan()["sessions"].as_array().unwrap().len(), 1);
    fs::remove_file(plain).unwrap();
    let data = f.scan();
    assert_eq!(data["sessions"].as_array().unwrap().len(), 1);
    assert!(data["sessions"][0]["file"]
        .as_str()
        .unwrap()
        .ends_with(".jsonl.zst"));
}

#[test]
fn redaction_preview_check_and_binary_safety() {
    let f = Fixture::new();
    let out = f.project.join(".agent-sessions");
    fs::create_dir_all(&out).unwrap();
    let input = b"\xff\0SESSDATA=abcdefghijklmnop\0\xfe";
    fs::write(out.join("binary.txt"), input).unwrap();
    assert_eq!(f.run(&["redact", "--check"]).status.code(), Some(1));
    assert_eq!(fs::read(out.join("binary.txt")).unwrap(), input);
    assert!(f.run(&["redact", "--dry-run"]).status.success());
    assert_eq!(fs::read(out.join("binary.txt")).unwrap(), input);
    assert!(f.run(&["redact"]).status.success());
    assert_eq!(
        fs::read(out.join("binary.txt")).unwrap(),
        b"\xff\0SESSDATA=[redacted]\0\xfe"
    );
    assert!(f.run(&["redact", "--check"]).status.success());
}
