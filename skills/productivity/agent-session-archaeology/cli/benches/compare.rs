use anyhow::{ensure, Context, Result};
use clap::Parser;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command, time::Instant};

#[derive(Parser)]
#[command(about = "Compare release binaries using isolated synthetic stores, never real logs")]
struct Args {
    before: PathBuf,
    after: PathBuf,
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u32).range(1..))]
    runs: u32,
    // Cargo adds this flag for harness-free benchmarks.
    #[arg(long, hide = true)]
    bench: bool,
}

struct Fixture {
    root: tempfile::TempDir,
    project: PathBuf,
    home: PathBuf,
    page: PathBuf,
}

impl Fixture {
    fn new() -> Result<Self> {
        let root = tempfile::Builder::new().prefix("search-bench-").tempdir()?;
        let project = root.path().join("project");
        let home = root.path().join("stores");
        let store = home.join(".codex/sessions");
        fs::create_dir_all(&project)?;
        fs::create_dir_all(&store)?;
        let mut header = json!({"type":"session_meta","payload":{"id":"synthetic","cwd":project}});
        let event = json!({"type":"response_item","payload":{"type":"message","role":"user",
            "content":[{"type":"input_text","text":"synthetic context ".repeat(64)}]}});
        fs::write(
            store.join("synthetic.jsonl"),
            format!("{header}\n{}", format!("{event}\n").repeat(30000)),
        )?;
        header["payload"]["id"] = json!("known");
        fs::write(store.join("known.jsonl"), format!("{header}\n{event}\n"))?;
        let page = root.path().join("page.txt");
        fs::write(
            &page,
            format!(
                "{} SESSDATA=abcdefghijklmnop\n",
                "synthetic context ".repeat(4096)
            )
            .repeat(100),
        )?;
        Ok(Self {
            root,
            project,
            home,
            page,
        })
    }

    fn run(&self, binary: &PathBuf, args: &[&str]) -> Result<(f64, Value)> {
        let started = Instant::now();
        let output = Command::new(binary)
            .arg("--json")
            .args(args)
            .current_dir(&self.project)
            .env("SESSION_ARCHAEOLOGY_HOME", &self.home)
            .output()
            .with_context(|| format!("cannot run {}", binary.display()))?;
        let elapsed = started.elapsed().as_secs_f64();
        ensure!(
            output.status.success(),
            "benchmark command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok((elapsed, serde_json::from_slice(&output.stdout)?))
    }
}

fn preserves_existing_fields(before: &Value, after: &Value) -> bool {
    match before {
        Value::Object(fields) => after.as_object().is_some_and(|next| {
            fields.iter().all(|(key, value)| {
                next.get(key)
                    .is_some_and(|v| preserves_existing_fields(value, v))
            })
        }),
        Value::Array(items) => after.as_array().is_some_and(|next| {
            items.len() == next.len()
                && items
                    .iter()
                    .zip(next)
                    .all(|(a, b)| preserves_existing_fields(a, b))
        }),
        _ => before == after,
    }
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    let middle = samples.len() / 2;
    if samples.len().is_multiple_of(2) {
        (samples[middle - 1] + samples[middle]) / 2.0
    } else {
        samples[middle]
    }
}

fn sorted_rows(value: &Value) -> Result<Vec<String>> {
    let mut rows = value
        .as_array()
        .context("inventory rows must be an array")?
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    rows.sort();
    Ok(rows)
}

fn main() -> Result<()> {
    let args = Args::parse();
    let binaries = [args.before.canonicalize()?, args.after.canonicalize()?];
    let fixture = Fixture::new()?;
    let page = fixture.page.to_str().context("fixture path is not UTF-8")?;
    let cases: [(&str, &[&str]); 3] = [
        ("scan_30000_events", &["sessions", "scan"]),
        (
            "resolve_one_of_two_sessions",
            &["sessions", "resolve", "--source", "codex", "--id", "known"],
        ),
        (
            "read_100_large_lines",
            &["read", "--file", page, "--from", "1", "--to", "100"],
        ),
    ];
    let mut report = json!({});
    for (name, command) in cases {
        let mut samples = [Vec::new(), Vec::new()];
        let mut expected = None;
        // Warm both binaries, then alternate their order to reduce timing bias.
        for run in 0..=args.runs {
            for index in if run.is_multiple_of(2) {
                [0, 1]
            } else {
                [1, 0]
            } {
                let (elapsed, output) = fixture.run(&binaries[index], command)?;
                let previous = expected.get_or_insert_with(|| output.clone());
                ensure!(
                    preserves_existing_fields(previous, &output),
                    "CLI output changed: {name}"
                );
                if run > 0 {
                    samples[index].push(elapsed);
                }
            }
        }
        let before = median(&mut samples[0]);
        let after = median(&mut samples[1]);
        report[name] = json!({"before_seconds":before, "after_seconds":after,
            "speedup":before / after, "preserved_existing_fields":true, "runs":args.runs});
    }
    let (_, inventory) = fixture.run(&binaries[1], &["sessions", "scan"])?;
    let baseline = fixture.root.path().join("inventory.json");
    fs::write(&baseline, serde_json::to_vec(&inventory)?)?;
    let expected = sorted_rows(&inventory["data"]["sessions"])?;
    let mut samples = Vec::new();
    for _ in 0..args.runs {
        let (elapsed, delta) = fixture.run(
            &binaries[1],
            &[
                "sessions",
                "scan",
                "--since",
                baseline.to_str().context("fixture path is not UTF-8")?,
            ],
        )?;
        ensure!(
            delta["data"]["sessions"] == json!([]) && delta["data"]["not_seen"] == json!([]),
            "unchanged scan returned changed/missing originals"
        );
        ensure!(
            sorted_rows(&delta["data"]["unchanged"])? == expected,
            "inventory rows changed"
        );
        samples.push(elapsed);
    }
    let elapsed = median(&mut samples);
    report["incremental_unchanged_scan"] = json!({"after_seconds":elapsed, "runs":args.runs,
        "speedup_vs_current_full_scan":report["scan_30000_events"]["after_seconds"].as_f64().unwrap() / elapsed,
        "same_inventory_rows":true});
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
