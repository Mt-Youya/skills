use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture {
    root: tempfile::TempDir,
    guide: PathBuf,
    session: PathBuf,
}

impl Fixture {
    fn new(records: &[(&str, Option<&str>)]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let guide = root.path().join("docs/avoiding-pitfalls-guide");
        fs::create_dir_all(&guide).unwrap();
        let mut topic = String::from("# 验证\n\n> 保留真实证据。\n\n");
        for (id, reference) in records {
            topic.push_str(&format!("### {id} · {id} title\n\n`batch` 2026-10-22 · `severity` high · `status` verified · {} `source` test/session · retry @1-4\n\n- Evidence follows.\n- 📎 excerpt: [evidence.gz](./evidence.gz) → {id}\n\n",reference.map_or(String::new(),|r|format!("`reference` {r} ·"))));
        }
        fs::write(guide.join("01-verification.md"), topic).unwrap();
        fs::write(
            guide.join("README.md"),
            "# Guide\n\nKeep this prose.\n\n<!-- BEGIN:auto-index -->\n<!-- END:auto-index -->\n",
        )
        .unwrap();
        let session = root.path().join("session.md");
        fs::write(
            &session,
            "old evidence\nold result\nnew evidence\nnew result\n",
        )
        .unwrap();
        Self {
            root,
            guide,
            session,
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_sarch"))
            .current_dir(self.root.path())
            .env("SESSION_ARCHAEOLOGY_HOME", self.root.path().join("stores"))
            .arg("--json")
            .args(args)
            .output()
            .unwrap()
    }
    fn add(&self, id: &str, range: &str) {
        let result = self.run(&[
            "evidence",
            "add",
            "--id",
            id,
            "--segment",
            &format!("{}:{range}", self.session.display()),
        ]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
    }
    fn archive(&self) -> PathBuf {
        self.guide.join("evidence.gz")
    }
    fn shown(&self, id: &str) -> String {
        let result = self.run(&["evidence", "show", "--id", id]);
        assert!(result.status.success());
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["data"]["text"]
            .as_str()
            .unwrap()
            .into()
    }
    fn packed(&self, text: &str) {
        let mut output = GzEncoder::new(Vec::new(), Compression::default());
        output.write_all(text.as_bytes()).unwrap();
        fs::write(self.archive(), output.finish().unwrap()).unwrap();
    }
}

#[test]
fn missing_archive_fails() {
    let f = Fixture::new(&[("VER-01", None)]);
    let result = f.run(&["evidence", "check"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stdout).contains("evidence.gz"));
}

#[test]
fn empty_archive_fails() {
    let f = Fixture::new(&[("VER-01", None)]);
    f.packed("# Evidence archive\n");
    assert_eq!(f.run(&["evidence", "check"]).status.code(), Some(1));
}

#[test]
fn independent_records_need_coverage() {
    let f = Fixture::new(&[("VER-01", None), ("DATA-01", None)]);
    f.add("VER-01", "1-2");
    let result = f.run(&["evidence", "check"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&result.stdout).contains("DATA-01"));
}

#[test]
fn invalid_segment_fails() {
    let f = Fixture::new(&[("VER-01", None)]);
    f.packed("<!-- section:VER-01 -->\n## VER-01 · title\n### Segment 1 · session lines 1–2 (2 lines)\nmissing digest\n");
    assert_eq!(f.run(&["evidence", "check"]).status.code(), Some(1));
}

#[test]
fn changed_digest_fails() {
    let f = Fixture::new(&[("VER-01", None)]);
    f.add("VER-01", "1-2");
    let mut text = String::new();
    GzDecoder::new(fs::File::open(f.archive()).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    f.packed(&text.replace("old evidence", "bad evidence"));
    assert_eq!(f.run(&["evidence", "check"]).status.code(), Some(1));
}

#[test]
fn references_reuse_independent_evidence() {
    let f = Fixture::new(&[("VER-01", None), ("TOOL-01", Some("VER-01"))]);
    f.add("VER-01", "1-2");
    assert!(f.run(&["evidence", "check"]).status.success());
}

#[test]
fn budget_keeps_exit_two() {
    let f = Fixture::new(&[("VER-01", None)]);
    f.add("VER-01", "1-2");
    assert_eq!(
        f.run(&["evidence", "check", "--budget-mb", "0"])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn damaged_gzip_fails() {
    let f = Fixture::new(&[("VER-01", None)]);
    fs::write(f.archive(), "bad gzip").unwrap();
    assert_eq!(f.run(&["evidence", "check"]).status.code(), Some(1));
}

#[test]
fn incremental_add_preserves_deduplicates_and_keeps_other_ids() {
    let f = Fixture::new(&[("VER-01", None), ("DATA-01", None)]);
    f.add("VER-01", "1-2");
    f.add("DATA-01", "1-2");
    let other = f.shown("DATA-01");
    f.add("VER-01", "3-4");
    let before = fs::read(f.archive()).unwrap();
    f.add("VER-01", "1-2");
    f.add("VER-01", "3-4");
    assert_eq!(fs::read(f.archive()).unwrap(), before);
    let shown = f.shown("VER-01");
    assert!(shown.contains("old evidence\nold result"));
    assert!(shown.contains("new evidence\nnew result"));
    assert_eq!(shown.matches("### Segment ").count(), 2);
    assert!(shown.contains("`segments` 2"));
    assert_eq!(f.shown("DATA-01"), other);
    assert!(f.run(&["evidence", "check"]).status.success());
}

#[test]
fn changed_content_at_same_coordinates_is_new_evidence() {
    let f = Fixture::new(&[("VER-01", None)]);
    f.add("VER-01", "1-2");
    fs::write(&f.session, "changed evidence\nchanged result\n").unwrap();
    f.add("VER-01", "1-2");
    let shown = f.shown("VER-01");
    assert!(shown.contains("old evidence"));
    assert!(shown.contains("changed evidence"));
    assert_eq!(shown.matches("### Segment ").count(), 2);
}

#[test]
fn equal_content_from_different_sources_or_ranges_is_retained() {
    let f = Fixture::new(&[("VER-01", None)]);
    let other = f.root.path().join("another-session.md");
    fs::write(&other, "old evidence\nold result\n").unwrap();
    fs::write(
        &f.session,
        "old evidence\nold result\nold evidence\nold result\n",
    )
    .unwrap();
    f.add("VER-01", "1-2");
    f.add("VER-01", "3-4");
    assert!(f
        .run(&[
            "evidence",
            "add",
            "--id",
            "VER-01",
            "--segment",
            &format!("{}:1-2", other.display())
        ])
        .status
        .success());
    assert_eq!(f.shown("VER-01").matches("### Segment ").count(), 3);
}

#[test]
fn legacy_spacing_and_trailing_blank_lines_survive() {
    let f = Fixture::new(&[("VER-01", None), ("DATA-01", None)]);
    f.add("VER-01", "1-5");
    f.add("DATA-01", "1-2");
    let mut text = String::new();
    GzDecoder::new(fs::File::open(f.archive()).unwrap())
        .read_to_string(&mut text)
        .unwrap();
    f.packed(&text.replace("lines)\n- sha256:", "lines)\n\n- sha256:"));
    assert!(f.run(&["evidence", "check"]).status.success());
    let before = fs::read(f.archive()).unwrap();
    f.add("VER-01", "1-5");
    assert_eq!(fs::read(f.archive()).unwrap(), before);
    f.add("VER-01", "3-4");
    assert!(f.run(&["evidence", "check"]).status.success());
}

#[test]
fn unreadable_or_damaged_archive_is_not_overwritten() {
    let f = Fixture::new(&[("VER-01", None)]);
    fs::create_dir(f.archive()).unwrap();
    let add = || {
        f.run(&[
            "evidence",
            "add",
            "--id",
            "VER-01",
            "--segment",
            &format!("{}:1-2", f.session.display()),
        ])
    };
    assert_eq!(add().status.code(), Some(1));
    fs::remove_dir(f.archive()).unwrap();
    fs::write(f.archive(), "damaged archive").unwrap();
    assert_eq!(add().status.code(), Some(1));
    assert_eq!(fs::read_to_string(f.archive()).unwrap(), "damaged archive");
}

#[test]
fn preview_does_not_write_and_new_excerpts_are_redacted() {
    let f = Fixture::new(&[("VER-01", None)]);
    fs::write(&f.session, "SESSDATA=abcdefghijklmnop\n").unwrap();
    let spec = format!("{}:1-1", f.session.display());
    assert!(f
        .run(&[
            "evidence",
            "add",
            "--id",
            "VER-01",
            "--segment",
            &spec,
            "--dry-run"
        ])
        .status
        .success());
    assert!(!f.archive().exists());
    f.add("VER-01", "1-1");
    assert!(f.shown("VER-01").contains("SESSDATA=[redacted]"));
    assert!(f.run(&["guide", "index", "--dry-run"]).status.success());
    assert_eq!(f.run(&["guide", "check"]).status.code(), Some(1));
    assert!(f.run(&["guide", "index"]).status.success());
    assert!(fs::read_to_string(f.guide.join("README.md"))
        .unwrap()
        .contains("Keep this prose."));
    assert!(f.run(&["guide", "check"]).status.success());
}

#[test]
fn inventory_preserves_source_and_reserves_reference_ids() {
    let f = Fixture::new(&[("VER-01", None), ("VER-09", Some("VER-01"))]);
    let result = f.run(&["guide", "inventory"]);
    assert!(result.status.success());
    let data: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(data["data"]["nextIds"]["VER"], "VER-10");
    assert_eq!(data["data"]["independentCount"], 1);
    assert_eq!(data["data"]["referenceCount"], 1);
    assert_eq!(
        data["data"]["pitfalls"][0]["source"],
        "test/session · retry @1-4"
    );
}
