use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

struct Fixture {
    _root: tempfile::TempDir,
    home: PathBuf,
    project: PathBuf,
    baseline: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let home = root.path().join("stores");
        let project = root.path().join("project");
        fs::create_dir_all(home.join(".codex/sessions")).unwrap();
        fs::create_dir_all(&project).unwrap();
        let baseline = root.path().join("inventory.json");
        Self {
            _root: root,
            home,
            project,
            baseline,
        }
    }
    fn run(&self, args: &[&str]) -> (bool, Value) {
        let output = Command::new(env!("CARGO_BIN_EXE_search"))
            .current_dir(&self.project)
            .env("SESSION_ARCHAEOLOGY_HOME", &self.home)
            .arg("--json")
            .args(args)
            .output()
            .unwrap();
        (
            output.status.success(),
            serde_json::from_slice(&output.stdout).unwrap(),
        )
    }
    fn session(&self, filename: &str, id: &str, body: &str) -> PathBuf {
        let file = self.home.join(".codex/sessions").join(filename);
        fs::write(&file, format!("{}\n{}\n", json!({"type":"session_meta","payload":{"id":id,"cwd":self.project}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":body}]}}))).unwrap();
        file
    }
    fn save(&self, value: &Value) {
        fs::write(&self.baseline, serde_json::to_vec(value).unwrap()).unwrap();
    }
    fn delta(&self) -> Value {
        let (ok, value) = self.run(&[
            "sessions",
            "scan",
            "--since",
            self.baseline.to_str().unwrap(),
        ]);
        assert!(ok, "{value}");
        value
    }
}

#[test]
fn unchanged_append_rewrite_and_missing_files_are_distinct() {
    let f = Fixture::new();
    let first = f.session("first.jsonl", "first", "initial");
    let second = f.session("second.jsonl", "second", "initial");
    let (_, baseline) = f.run(&["sessions", "scan"]);
    f.save(&baseline);
    let delta = f.delta();
    assert_eq!(delta["data"]["sessions"].as_array().unwrap().len(), 0);
    assert_eq!(delta["data"]["unchanged"].as_array().unwrap().len(), 2);
    assert_eq!(delta["data"]["semantic_review"], "not_reviewed");
    f.save(&delta); // Chained incremental inventories retain all unchanged originals.
    let mut text = fs::read_to_string(&first).unwrap();
    text.push_str("{\"type\":\"ignored but valid\"}\n");
    fs::write(&first, text).unwrap();
    let delta = f.delta();
    assert_eq!(delta["data"]["sessions"][0]["id"], "first");
    assert_eq!(delta["data"]["unchanged"][0]["id"], "second");
    f.save(&delta);
    // Same-size replacement must invalidate the metadata cache too.
    f.session("first.jsonl", "first", "changed");
    fs::remove_file(&second).unwrap();
    let delta = f.delta();
    assert_eq!(delta["data"]["sessions"][0]["id"], "first");
    assert!(delta["data"]["unchanged"].as_array().unwrap().is_empty());
    assert_eq!(delta["data"]["not_seen"][0]["id"], "second");
    f.save(&delta);
    let delta = f.delta();
    assert!(delta["data"]["sessions"].as_array().unwrap().is_empty());
    assert_eq!(delta["data"]["not_seen"][0]["id"], "second");
    f.session("new.jsonl", "new", "newly discovered");
    let delta = f.delta();
    assert_eq!(delta["data"]["sessions"][0]["id"], "new");
}

#[test]
fn old_or_damaged_inventory_cannot_hide_parse_work() {
    let f = Fixture::new();
    let file = f.session("one.jsonl", "one", "body");
    let (_, mut baseline) = f.run(&["sessions", "scan"]);
    baseline["data"]["sessions"][0]
        .as_object_mut()
        .unwrap()
        .remove("file_state");
    f.save(&baseline);
    assert_eq!(f.delta()["data"]["sessions"].as_array().unwrap().len(), 1);
    let mut text = fs::read_to_string(file).unwrap();
    text.push_str("{broken\n");
    fs::write(f.home.join(".codex/sessions/one.jsonl"), text).unwrap();
    let (ok, baseline) = f.run(&["sessions", "scan"]);
    assert!(!ok);
    f.save(&baseline);
    let (ok, delta) = f.run(&["sessions", "scan", "--since", f.baseline.to_str().unwrap()]);
    assert!(!ok);
    assert_eq!(delta["data"]["sessions"][0]["parse_errors"], json!([3]));
    assert!(delta["data"]["unchanged"].as_array().unwrap().is_empty());
}

#[test]
fn wrong_project_and_content_search_baselines_are_rejected() {
    let f = Fixture::new();
    f.session("one.jsonl", "one", "body");
    let (_, mut baseline) = f.run(&["sessions", "scan"]);
    baseline["data"]["content_search"] = json!(true);
    f.save(&baseline);
    assert!(
        !f.run(&["sessions", "scan", "--since", f.baseline.to_str().unwrap()])
            .0
    );
    baseline["data"]["content_search"] = json!(false);
    baseline["data"]["project"] = json!("/another-project");
    f.save(&baseline);
    assert!(
        !f.run(&["sessions", "scan", "--since", f.baseline.to_str().unwrap()])
            .0
    );
    assert!(
        !f.run(&[
            "sessions",
            "scan",
            "--since",
            f.baseline.to_str().unwrap(),
            "--content"
        ])
        .0
    );
}

#[test]
fn targeted_resolve_preserves_ambiguity_and_does_not_match_filenames() {
    let f = Fixture::new();
    f.session("misleading.jsonl", "actual-id", "body");
    f.session("actual-id.jsonl", "other-id", "unrelated");
    let (ok, result) = f.run(&[
        "sessions",
        "resolve",
        "--source",
        "codex",
        "--id",
        "actual-id",
    ]);
    assert!(ok);
    assert!(result["data"]["file"]
        .as_str()
        .unwrap()
        .ends_with("misleading.jsonl"));
    f.session("duplicate.jsonl", "actual-id", "duplicate");
    assert!(
        !f.run(&[
            "sessions",
            "resolve",
            "--source",
            "codex",
            "--id",
            "actual-id"
        ])
        .0
    );
}

#[test]
fn dsh_registry_ownership_is_rechecked_for_cached_files() {
    let f = Fixture::new();
    let registry = f.home.join(".dsh/storages/workspace.json");
    fs::create_dir_all(registry.parent().unwrap()).unwrap();
    fs::write(
        &registry,
        json!({"tables":{"workspaces":{"one":{"path":f.project,"sessionIds":["session-owned"]}}}})
            .to_string(),
    )
    .unwrap();
    let path = f.home.join(".dsh/sessions/owned/session.v1.jsonl.zstd");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let text = format!(
        "{}\n{}\n",
        json!({"type":"session","id":"owned","cwd":"/external"}),
        json!({"type":"user/message","data":{"content":"question"}})
    );
    fs::write(&path, zstd::stream::encode_all(text.as_bytes(), 1).unwrap()).unwrap();
    let (_, baseline) = f.run(&["sessions", "scan"]);
    f.save(&baseline);
    let delta = f.delta();
    assert_eq!(delta["data"]["registry"]["dsh_found"], 1);
    assert_eq!(delta["data"]["unchanged"].as_array().unwrap().len(), 1);
    fs::write(registry, "{}").unwrap();
    let delta = f.delta();
    assert!(delta["data"]["sessions"].as_array().unwrap().is_empty());
    assert!(delta["data"]["unchanged"].as_array().unwrap().is_empty());
    assert_eq!(delta["data"]["not_seen"][0]["id"], "owned");
}
