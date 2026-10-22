use crate::files;
use anyhow::{Context, Result};
use regex::bytes::{Captures, Regex};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path, sync::LazyLock};

struct Rule {
    name: &'static str,
    re: Regex,
    replacement: &'static str,
}

static RULES: LazyLock<Vec<Rule>> = LazyLock::new(|| {
    let names = "SESSDATA|bili_jct|DedeUserID__ckMd5|sec_ck|buvid3|session_?id|sessionid|token|jwt|csrf_?token|csrftoken|x-?api-?key|api_?key|apikey|access_?token|refresh_?token|client_?secret|aws_?access_?key_?id|aws_?secret_?access_?key|secret_?access_?key|private_?key|password|passwd|pwd";
    let value = r"[A-Za-z0-9%._~+/=\\-]{12,}";
    let providers = r"gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|sk-(?:proj-|svcacct-)?[A-Za-z0-9_-]{20,}|(?:sk|rk)_(?:live|test)_[A-Za-z0-9]{16,}|AIza[A-Za-z0-9_-]{30,}|glpat-[A-Za-z0-9_-]{16,}|npm_[A-Za-z0-9]{30,}|SG\.[A-Za-z0-9_-]{16,}\.[A-Za-z0-9_-]{16,}|dop_v1_[a-f0-9]{40,}|hf_[A-Za-z0-9]{30,}|pypi-[A-Za-z0-9_-]{30,}";
    let specs = vec![
        ("name=value pair", format!(r#"(?i)((?:{names})["']?\s*[=:]\s*["']?)({value})"#), "${1}[redacted]"),
        ("suffixed secret name", format!(r#"(?i)((?:[A-Za-z0-9]+[_-])+(?:secret|key|token|password|passwd|pwd)["']?\s*[=:]\s*["']?)({value})"#), "${1}[redacted]"),
        ("cookie identifier", r#"(?i)((?:DedeUserID|buvid3)["']?\s*[=:]\s*["']?)([A-Za-z0-9%._~+/=\\-]{6,})"#.into(), "${1}[redacted]"),
        ("JSON cookie object", format!(r#"(?i)("name"\s*:\s*"(?:{names}|DedeUserID|buvid3)"[\s\S]{{0,160}}?"value"\s*:\s*")([^"\[][^\"]{{5,}})"#), "${1}[redacted]"),
        ("authorization header", r#"(?im)(^|[\s"'(])((?:proxy-)?authorization\s*:\s*)[^\s"'\[][^\r\n"']{7,}"#.into(), "${1}${2}[redacted]"),
        ("bearer token", r"(?i)(\bBearer\s+)([A-Za-z0-9\-._~+/]{16,}=*)".into(), "${1}[redacted]"),
        ("basic auth", r"(?i)(\bBasic\s+)([A-Za-z0-9+/]{16,}=*)".into(), "${1}[redacted]"),
        ("cookie header", r#"(?im)(^|[\s"'(])((?:set-)?cookie\s*:\s*)[^\s"'\[][^\r\n"']{7,}"#.into(), "${1}${2}[redacted]"),
        ("provider token", format!(r"\b(?:{providers})"), "[redacted]"),
        ("bare JWT", r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}".into(), "[redacted]"),
        ("connection string password", r"(?i)(\b[a-z][a-z0-9+.\-]{1,20}://)([^/\s@:]{0,64}):([^@\s/]{4,})@".into(), "${1}${2}:[redacted]@"),
        ("aws access key id", r"\bAKIA[0-9A-Z]{16}\b".into(), "[redacted]"),
        ("private key block", r"-----BEGIN (?:RSA |OPENSSH |EC |PGP )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |OPENSSH |EC |PGP )?PRIVATE KEY-----".into(), "[redacted private key]"),
    ];
    specs
        .into_iter()
        .map(|(name, pattern, replacement)| Rule {
            name,
            re: regex::bytes::RegexBuilder::new(&pattern)
                .unicode(false)
                .build()
                .expect("built-in redaction rule"),
            replacement,
        })
        .collect()
});

pub fn bytes(input: &[u8]) -> (Vec<u8>, BTreeMap<String, usize>) {
    let mut output = input.to_vec();
    let mut counts = BTreeMap::new();
    for rule in RULES.iter() {
        output = rule
            .re
            .replace_all(&output, |captures: &Captures<'_>| {
                let mut result = Vec::new();
                captures.expand(rule.replacement.as_bytes(), &mut result);
                if result != captures[0] {
                    *counts.entry(rule.name.into()).or_insert(0) += 1;
                }
                result
            })
            .into_owned();
    }
    (output, counts)
}

pub fn text(input: &str) -> String {
    String::from_utf8(bytes(input.as_bytes()).0).expect("redaction preserves UTF-8")
}

pub fn directory(dir: &Path, dry_run: bool) -> Result<Value> {
    let mut files = 0;
    let mut touched = 0;
    let mut changed = 0;
    let mut counts = BTreeMap::<String, usize>::new();
    let extensions = [
        "md",
        "markdown",
        "json",
        "jsonl",
        "txt",
        "log",
        "csv",
        "tsv",
        "yml",
        "yaml",
        "toml",
        "ini",
        "env",
        "properties",
        "mjs",
        "js",
        "cjs",
        "ts",
        "ps1",
        "sh",
        "py",
    ];
    for path in files::walk(dir, usize::MAX)? {
        if path.file_name().is_some_and(|name| name == "coverage.json") {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !extensions.contains(&ext.as_str()) && path.file_name().is_none_or(|n| n != ".env") {
            continue;
        }
        files += 1;
        let input = fs::read(&path).with_context(|| format!("cannot check {}", path.display()))?;
        let (output, hits) = bytes(&input);
        if input == output {
            continue;
        }
        touched += 1;
        for (name, count) in hits {
            *counts.entry(name).or_default() += count;
        }
        if !dry_run {
            files::atomic_write(&path, &output)?;
            changed += 1;
        }
    }
    Ok(
        json!({"files":files,"touched":touched,"changed":changed,"counts":counts,"unreadable":0,"missing":false,"dry_run":dry_run}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redaction_preserves_context_and_converges() {
        let input = "Session: real-session-id\nSession_id=abcdefghijklmnop\nCookie: foo=abcdefghi\nAuthorization: Basic abcdefghijklmnop==\npostgres://user:long-password@host/db\nSESSDATA=abcdefghijklmnop\nWEBHOOK_SECRET=abcdefghijklmnop\n\"name\":\"bili_jct\",\"value\":\"abcdefghijk\"\nghp_abcdefghijklmnopqrstuvwxyz\n";
        let output = text(input);
        assert!(output.contains("Session: real-session-id"));
        assert!(!output.contains("abcdefghijklmnop"));
        assert!(!output.contains("long-password"));
        assert!(!output.contains("abcdefghijk"));
        assert_eq!(output, text(&output));
    }
    #[test]
    fn binary_bytes_survive() {
        let input = b"\xff\0SESSDATA=abcdefghijklmnop\0\xfe";
        assert_eq!(bytes(input).0, b"\xff\0SESSDATA=[redacted]\0\xfe");
    }
}
