use crate::files;
use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const BEGIN: &str = "<!-- BEGIN:auto-index -->";
const END: &str = "<!-- END:auto-index -->";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pitfall {
    pub id: String,
    pub name: String,
    pub severity: String,
    pub status: String,
    pub batch: String,
    pub source: String,
    pub has_evidence: bool,
    pub reference: String,
    pub topic: String,
    pub file: String,
}

#[derive(Serialize)]
pub struct Topic {
    pub file: String,
    pub title: String,
    pub blurb: Option<String>,
    pub pitfalls: Vec<Pitfall>,
}

pub struct Inventory {
    pub topics: Vec<Topic>,
    pub all: Vec<Pitfall>,
}

pub fn field(meta: &str, key: &str) -> String {
    let label = format!("`{key}`");
    let Some(start) = meta.find(&label) else {
        return "—".into();
    };
    let value = &meta[start + label.len()..];
    let end = Regex::new(r"\s*·\s*`(?:batch|severity|status|source|reference)`(?:\s|$)")
        .unwrap()
        .find(value)
        .map_or(value.len(), |m| m.start());
    value[..end].trim().into()
}

pub fn collect(guide: &Path) -> Result<Inventory> {
    let topic_re = Regex::new(r"^\d{2}-.+\.md$")?;
    let heading_re = Regex::new(r"^###\s+([A-Za-z]+-\d+)\s*·\s*(.+?)\s*$")?;
    let title_re = Regex::new(r"^\d+\s*·\s*")?;
    let mut names = Vec::new();
    if guide.exists() {
        for entry in fs::read_dir(guide)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type()?.is_file() && topic_re.is_match(&name) {
                names.push(name);
            }
        }
    }
    names.sort();
    let mut topics = Vec::new();
    let mut all = Vec::new();
    for name in names {
        let text = fs::read_to_string(guide.join(&name))?;
        let lines: Vec<_> = text.split('\n').collect();
        let title = lines
            .iter()
            .find_map(|line| line.strip_prefix("# "))
            .map(|s| title_re.replace(s.trim(), "").into_owned())
            .unwrap_or_else(|| name.trim_end_matches(".md").into());
        let blurb = lines
            .iter()
            .take_while(|line| !line.starts_with("### "))
            .find_map(|line| line.trim().strip_prefix('>').map(|s| s.trim().to_string()));
        let mut pitfalls = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let Some(caps) = heading_re.captures(line) else {
                continue;
            };
            let mut meta = String::new();
            for line in lines.iter().skip(i + 1).take(5) {
                if line.starts_with("- ") {
                    break;
                }
                meta.push(' ');
                meta.push_str(line);
                if meta.contains("`source`") {
                    break;
                }
            }
            let body = lines
                .iter()
                .skip(i + 1)
                .take_while(|line| !line.starts_with("### ") && **line != "---")
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            pitfalls.push(Pitfall {
                id: caps[1].into(),
                name: caps[2].trim().into(),
                severity: field(&meta, "severity"),
                status: field(&meta, "status"),
                batch: field(&meta, "batch"),
                source: field(&meta, "source"),
                has_evidence: body.contains('📎'),
                reference: field(&meta, "reference"),
                topic: title.clone(),
                file: name.clone(),
            });
        }
        all.extend(pitfalls.clone());
        topics.push(Topic {
            file: name,
            title,
            blurb,
            pitfalls,
        });
    }
    let ids: BTreeMap<_, _> = all
        .iter()
        .map(|p| (p.id.clone(), p.reference.clone()))
        .collect();
    if ids.len() != all.len() {
        bail!("Duplicate pitfall IDs in the guide");
    }
    for record in &mut all {
        if record.reference == "—" {
            continue;
        }
        let reference = record.reference.replace('`', "");
        if reference == record.id || ids.get(&reference).is_none_or(|v| v != "—") {
            bail!("Invalid reference for {}: {}", record.id, record.reference);
        }
        record.reference = reference;
    }
    Ok(Inventory { topics, all })
}

pub fn inventory(guide: &Path) -> Result<Value> {
    let data = collect(guide)?;
    let mut highest = BTreeMap::<String, u64>::new();
    for p in &data.all {
        if let Some((prefix, n)) = p.id.rsplit_once('-') {
            if let Ok(n) = n.parse::<u64>() {
                highest
                    .entry(prefix.into())
                    .and_modify(|old| *old = (*old).max(n))
                    .or_insert(n);
            }
        }
    }
    let next: BTreeMap<_, _> = highest
        .into_iter()
        .map(|(p, n)| (p.clone(), format!("{p}-{:02}", n + 1)))
        .collect();
    let topics: Vec<_> = data.topics.iter().map(|t| json!({"file":t.file,"title":t.title,"pitfalls":t.pitfalls.iter().filter(|p| p.reference=="—").count(),"references":t.pitfalls.iter().filter(|p| p.reference!="—").count()})).collect();
    let ids: BTreeSet<_> = data.all.iter().map(|p| &p.id).collect();
    Ok(
        json!({"guide":guide,"topics":topics,"independentCount":data.all.iter().filter(|p| p.reference=="—").count(),"referenceCount":data.all.iter().filter(|p| p.reference!="—").count(),"existingIds":ids,"nextIds":next,"pitfalls":data.all}),
    )
}

fn cell(value: &str) -> String {
    value.trim().replace('|', "\\|")
}

fn render(data: &Inventory) -> String {
    let independent: Vec<_> = data.all.iter().filter(|p| p.reference == "—").collect();
    let references = data.all.len() - independent.len();
    let mut out = vec![
        "## 主题索引".into(),
        "".into(),
        "| 主题 | 独立记录 | 引用入口 | 覆盖 |".into(),
        "| --- | --- | --- | --- |".into(),
    ];
    for t in &data.topics {
        if t.pitfalls.is_empty() {
            continue;
        }
        let count = t.pitfalls.iter().filter(|p| p.reference == "—").count();
        out.push(format!(
            "| [{}](./{}) | {} | {} | {} |",
            cell(&t.title),
            t.file,
            count,
            t.pitfalls.len() - count,
            cell(t.blurb.as_deref().unwrap_or(""))
        ));
    }
    out.extend([
        "".into(),
        format!(
            "**{} 条独立记录，{} 个去重引用入口，分为 {} 个主题。**",
            independent.len(),
            references,
            data.topics
                .iter()
                .filter(|t| !t.pitfalls.is_empty())
                .count()
        ),
        "".into(),
        "## 全部记录".into(),
        "".into(),
        "| ID | 坑点 | 主题 | 严重度 | 状态 |".into(),
        "| --- | --- | --- | --- | --- |".into(),
    ]);
    let mut all = data.all.clone();
    all.sort_by(|a, b| a.id.cmp(&b.id));
    for p in all {
        out.push(format!(
            "| [{}](./{}#{}) | {} | {} | {} | {}{} |",
            p.id,
            p.file,
            p.id.to_lowercase(),
            cell(&p.name),
            cell(&p.topic),
            p.severity,
            p.status,
            if p.reference == "—" {
                String::new()
            } else {
                format!("（引用 {}）", p.reference)
            }
        ));
    }
    let mut severities = BTreeMap::<String, usize>::new();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut batches = BTreeSet::new();
    for p in &independent {
        *severities.entry(p.severity.clone()).or_default() += 1;
        *statuses.entry(p.status.clone()).or_default() += 1;
        if p.batch != "—" {
            batches.insert(format!("`{}`", p.batch));
        }
    }
    let counts = |items: BTreeMap<String, usize>| {
        items
            .into_iter()
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(" · ")
    };
    out.extend([
        "".into(),
        "## 统计".into(),
        "".into(),
        format!(
            "- 独立记录：**{}**；去重引用入口：**{}**",
            independent.len(),
            references
        ),
        format!(
            "- 批次：{}",
            if batches.is_empty() {
                "无".into()
            } else {
                batches.into_iter().collect::<Vec<_>>().join(", ")
            }
        ),
        format!("- 严重度：{}", counts(severities)),
        format!("- 状态：{}", counts(statuses)),
        format!(
            "- 带归档证据：{}",
            independent.iter().filter(|p| p.has_evidence).count()
        ),
        "".into(),
    ]);
    out.join("\n")
}

pub fn index(guide: &Path, check: bool, dry_run: bool) -> Result<Value> {
    let data = collect(guide)?;
    if data.all.is_empty() {
        bail!("No pitfall records found under {}", guide.display());
    }
    let path = guide.join("README.md");
    let existing = fs::read_to_string(&path)
        .context("README.md needs BEGIN:auto-index / END:auto-index markers")?;
    let b = existing
        .find(BEGIN)
        .context("README is missing BEGIN:auto-index")?;
    let e = existing
        .find(END)
        .context("README is missing END:auto-index")?;
    if e < b {
        bail!("README index markers are reversed");
    }
    let next = format!(
        "{}\n{}{}",
        &existing[..b + BEGIN.len()],
        render(&data),
        &existing[e..]
    );
    let changed = next != existing;
    if check && changed {
        bail!("README index is stale; run guide index");
    }
    if changed && !check && !dry_run {
        files::atomic_write(&path, next.as_bytes())?;
    }
    Ok(
        json!({"path":path,"changed":changed,"dry_run":dry_run,"pitfalls":data.all.len(),"topics":data.topics.len()}),
    )
}

pub fn check_chapters(guide: &Path) -> Result<()> {
    for (index, topic) in collect(guide)?.topics.iter().enumerate() {
        if !topic.file.starts_with(&format!("{:02}-", index + 1)) {
            bail!("chapter numbering has gaps at {}; compact topic filenames and repair repository links", topic.file);
        }
        if topic.pitfalls.is_empty() {
            bail!("empty placeholder chapter: {}", topic.file);
        }
    }
    Ok(())
}
