use crate::{files, guide, redact};
use anyhow::{bail, Context, Result};
use flate2::{read::MultiGzDecoder, write::GzEncoder, Compression};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

type Archive = BTreeMap<String, String>;
const NAME: &str = "evidence.gz";

pub fn read(guide: &Path) -> Result<Archive> {
    let bytes = match fs::read(guide.join(NAME)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(e).context("cannot read evidence.gz"),
    };
    let mut text = String::new();
    MultiGzDecoder::new(bytes.as_slice())
        .read_to_string(&mut text)
        .context("cannot read evidence.gz")?;
    let re = Regex::new(r"(?m)^<!-- section:([A-Za-z]+-?\d+) -->$")?;
    let markers: Vec<_> = re.captures_iter(&text).collect();
    let mut sections = BTreeMap::new();
    for (i, marker) in markers.iter().enumerate() {
        let start = marker.get(0).unwrap().end();
        let end = markers
            .get(i + 1)
            .map_or(text.len(), |m| m.get(0).unwrap().start());
        let body = &text[start..end];
        let body = if i + 1 < markers.len() {
            body.strip_suffix('\n').unwrap_or(body)
        } else {
            body
        };
        if sections.insert(marker[1].into(), body.into()).is_some() {
            bail!("duplicate evidence section: {}", &marker[1]);
        }
    }
    Ok(sections)
}

#[derive(Serialize)]
pub struct Segment {
    session: String,
    lines: [usize; 2],
    sha256: String,
}

pub fn segments(body: &str) -> Vec<Segment> {
    let header = Regex::new(r"(?m)^### Segment \d+ · ").unwrap();
    let ranges: Vec<_> = header.find_iter(body).map(|m| m.start()).collect();
    let re = Regex::new(r"\A### Segment \d+ · (.+) lines (\d+)–(\d+) \((\d+) lines\)\n(?:\n|- Original export: [^\n]*\n)*- sha256: ([a-f0-9]{64})\n\n([\s\S]*)\z").unwrap();
    ranges
        .iter()
        .enumerate()
        .filter_map(|(i, from)| {
            let part = &body[*from..ranges.get(i + 1).copied().unwrap_or(body.len())];
            let m = re.captures(part)?;
            let from: usize = m[2].parse().ok()?;
            let to: usize = m[3].parse().ok()?;
            let count: usize = m[4].parse().ok()?;
            let raw = m.get(6)?.as_str();
            let text = raw.strip_suffix('\n').unwrap_or(raw);
            if text.trim().is_empty()
                || from == 0
                || to < from
                || to - from + 1 != count
                || text.split('\n').count() != count
                || files::hash(text.as_bytes()) != m[5]
            {
                return None;
            }
            Some(Segment {
                session: m[1].into(),
                lines: [from, to],
                sha256: m[5].into(),
            })
        })
        .collect()
}

fn title(id: &str, body: &str) -> String {
    Regex::new(r"(?m)^##\s+[^\n·]*·\s*(.+)$")
        .unwrap()
        .captures(body)
        .map_or_else(|| id.into(), |m| m[1].trim().into())
}

fn render(archive: &Archive) -> String {
    let mut text = format!("# Evidence archive\n\n- Sections: {}\n- Updated: {}\n- Kept in one file on purpose: one section per pitfall, still greppable after `unpack`.\n- Print a single section without unpacking: `sarch evidence show --id <ID>`\n\n| ID | Pitfall | Segments | Session lines |\n| --- | --- | --- | --- |\n",archive.len(),files::now().to_rfc3339_opts(chrono::SecondsFormat::Millis,true));
    for (id, body) in archive {
        let excerpts = segments(body);
        let ranges = excerpts
            .iter()
            .map(|s| format!("{}-{}", s.lines[0], s.lines[1]))
            .collect::<Vec<_>>()
            .join(", ");
        text.push_str(&format!(
            "| {id} | {} | {} | {} |\n",
            title(id, body).replace('|', "\\|"),
            excerpts.len(),
            if ranges.is_empty() { "—" } else { &ranges }
        ));
    }
    text.push('\n');
    let bodies = archive
        .iter()
        .map(|(id, body)| format!("<!-- section:{id} -->\n{}", body.trim_start_matches('\n')))
        .collect::<Vec<_>>()
        .join("\n");
    text.push_str(&bodies);
    text
}

fn write(guide: &Path, archive: &Archive) -> Result<()> {
    let mut gzip = GzEncoder::new(Vec::new(), Compression::best());
    gzip.write_all(render(archive).as_bytes())?;
    files::atomic_write(&guide.join(NAME), &gzip.finish()?)?;
    Ok(())
}

fn validate_id(id: &str) -> Result<String> {
    let id = id.trim().to_uppercase();
    if !Regex::new(r"^[A-Z]+-?\d+$")?.is_match(&id) {
        bail!("--id must look like VER-02 or P008");
    }
    Ok(id)
}

pub struct Add<'a> {
    pub guide: &'a Path,
    pub id: &'a str,
    pub title: Option<&'a str>,
    pub anchor: Option<&'a str>,
    pub requested: Vec<String>,
    pub budget: f64,
    pub dry_run: bool,
}

pub fn add(args: Add<'_>) -> Result<Value> {
    if args.requested.is_empty() {
        bail!("missing --segment FILE:FROM-TO (or --session and --lines)");
    }
    budget(args.budget)?;
    let id = validate_id(args.id)?;
    let mut archive = read(args.guide)?;
    let existing = archive.get(&id).cloned().unwrap_or_default();
    let mut seen: BTreeSet<_> = segments(&existing)
        .iter()
        .map(|s| (s.session.clone(), s.lines, s.sha256.clone()))
        .collect();
    let count = Regex::new(r"(?m)^### Segment ")?
        .find_iter(&existing)
        .count();
    let range_re = Regex::new(r"^(.+):(\d+)\s*-\s*(\d+)$")?;
    let reserved_re = Regex::new(r"(?m)^(?:<!-- section:|### Segment \d+ · )")?;
    let personal_re = Regex::new(r"(?:[A-Za-z]:\\Users\\|/Users/)")?;
    let mut body = String::new();
    let mut added = 0;
    let mut warnings = Vec::<String>::new();
    for requested in args.requested {
        let m = range_re
            .captures(&requested)
            .context("--segment must look like FILE:FROM-TO")?;
        let file = PathBuf::from(&m[1]);
        let from = m[2].parse::<usize>()?;
        let to = m[3].parse::<usize>()?;
        let text = redact::text(&files::slice(&files::read_text(&file)?, from, to)?);
        if text.trim().is_empty() {
            bail!("evidence excerpt is empty");
        }
        // Reserved archive delimiters cannot be embedded verbatim in an excerpt.
        if reserved_re.is_match(&text) {
            bail!("excerpt contains reserved archive delimiters; choose a narrower range");
        }
        let n = text.split('\n').count();
        let filename = file
            .file_name()
            .context("segment needs a filename")?
            .to_string_lossy()
            .to_string();
        let session = filename
            .strip_suffix(".md")
            .or_else(|| filename.strip_suffix(".jsonl"))
            .unwrap_or(&filename)
            .to_string();
        let sha = files::hash(text.as_bytes());
        if !seen.insert((session.clone(), [from, from + n - 1], sha.clone())) {
            continue;
        }
        added += 1;
        if personal_re.is_match(&text) {
            warnings
                .push("excerpt contains a personal path; rewrite incidental paths as ~/…".into());
        }
        body.push_str(&format!(
            "### Segment {} · {session} lines {from}–{} ({n} lines)\n- sha256: {sha}\n\n{text}\n",
            count + added,
            from + n - 1
        ));
    }
    if added > 0 {
        let header = if existing.is_empty() {
            let title = args.title.unwrap_or(&id);
            let anchor = args
                .anchor
                .map_or(String::new(), |a| format!(" · `anchor` {a}"));
            format!("<a id=\"{}\"></a>\n\n## {id} · {title}\n\n`pitfall` {title} · `segments` {added} · `added` {}{anchor}\n\n",id.to_lowercase(),files::now().format("%Y-%m-%d"))
        } else {
            Regex::new(r"`segments` \d+")?
                .replace(&existing, format!("`segments` {}", count + added))
                .into_owned()
        };
        archive.insert(id.clone(), format!("{header}{body}"));
        if !args.dry_run {
            write(args.guide, &archive)?;
        }
    }
    let raw = render(&archive).len();
    if raw > 512 * 1024 {
        warnings.push("archive exceeds 512 KB uncompressed; trim excerpts".into());
    }
    let mut result = measure(args.guide, args.budget)?;
    if args.dry_run && added > 0 {
        let mut gzip = GzEncoder::new(Vec::new(), Compression::best());
        gzip.write_all(render(&archive).as_bytes())?;
        let planned = gzip.finish()?.len() as u64;
        let previous = fs::metadata(args.guide.join(NAME)).map_or(0, |m| m.len());
        let packed = result["archive_bytes"].as_u64().unwrap() - previous + planned;
        let total = result["prose_bytes"].as_u64().unwrap() + packed;
        result["archive_bytes"] = json!(packed);
        result["total_bytes"] = json!(total);
        result["over_budget"] = json!(total as f64 > args.budget * 1024.0 * 1024.0);
    }
    result["id"] = json!(id);
    result["added"] = json!(added);
    result["segments"] = json!(count + added);
    result["dry_run"] = json!(args.dry_run);
    result["warnings"] = json!(warnings);
    Ok(result)
}

fn budget(value: f64) -> Result<()> {
    if !value.is_finite() || value < 0.0 {
        bail!("--budget-mb must be a finite nonnegative number");
    }
    Ok(())
}

pub fn measure(guide: &Path, budget_mb: f64) -> Result<Value> {
    budget(budget_mb)?;
    let mut prose = 0u64;
    let mut packed = 0u64;
    if guide.exists() {
        for entry in fs::read_dir(guide)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let size = entry.metadata()?.len();
            if entry.path().extension().is_some_and(|e| e == "gz") {
                packed += size;
            } else {
                prose += size;
            }
        }
    }
    Ok(
        json!({"archive":guide.join(NAME),"prose_bytes":prose,"archive_bytes":packed,"total_bytes":prose+packed,"budget_mb":budget_mb,"over_budget":(prose+packed) as f64>budget_mb*1024.0*1024.0}),
    )
}

pub fn check(guide: &Path, budget_mb: f64) -> Result<Value> {
    if !guide.join(NAME).is_file() {
        bail!("missing required evidence.gz");
    }
    let archive = read(guide)?;
    if archive.is_empty() {
        bail!("evidence.gz is empty (no evidence sections)");
    }
    let missing: Vec<_> = guide::collect(guide)?
        .all
        .into_iter()
        .filter(|p| {
            p.reference == "—"
                && archive
                    .get(&p.id)
                    .is_none_or(|body| segments(body).is_empty())
        })
        .map(|p| p.id)
        .collect();
    if !missing.is_empty() {
        bail!("missing valid evidence segment(s): {}", missing.join(", "));
    }
    let mut report = measure(guide, budget_mb)?;
    report["sections"] = json!(archive.len());
    Ok(report)
}

pub fn show(guide: &Path, id: &str) -> Result<String> {
    let id = validate_id(id)?;
    let archive = read(guide)?;
    let body = archive
        .get(&id)
        .with_context(|| format!("no such evidence section: {id}"))?;
    Ok(format!(
        "<!-- section:{id} -->\n{}",
        body.trim_start_matches('\n')
    ))
}

pub fn list(guide: &Path) -> Result<Value> {
    let archive = read(guide)?;
    Ok(
        json!({"archive":guide.join(NAME),"sections":archive.iter().map(|(id,body)|json!({"id":id,"title":title(id,body),"segments":segments(body)})).collect::<Vec<_>>()}),
    )
}

pub fn remove(guide: &Path, ids: &[String], dry_run: bool) -> Result<Value> {
    let mut archive = read(guide)?;
    let mut removed = Vec::new();
    for id in ids {
        let id = validate_id(id)?;
        if archive.remove(&id).is_some() {
            removed.push(id);
        }
    }
    if removed.is_empty() {
        bail!("nothing removed: no matching evidence sections");
    }
    if !dry_run {
        write(guide, &archive)?;
    }
    Ok(json!({"removed":removed,"dry_run":dry_run}))
}

pub fn unpack(guide: &Path, out: Option<&Path>, dry_run: bool) -> Result<Value> {
    let text = render(&read(guide)?);
    let out = out.map_or_else(
        || {
            std::env::temp_dir().join(format!(
                "pitfall-evidence-{}.md",
                files::now().timestamp_millis()
            ))
        },
        Path::to_path_buf,
    );
    let out = files::absolute(&out)?;
    if !dry_run {
        files::atomic_write(&out, text.as_bytes())?;
    }
    Ok(json!({"path":out,"bytes":text.len(),"dry_run":dry_run}))
}
