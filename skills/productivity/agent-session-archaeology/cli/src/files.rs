use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub fn absolute(path: &Path) -> Result<PathBuf> {
    let full = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut result = PathBuf::new();
    for component in full.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            other => result.push(other.as_os_str()),
        }
    }
    // Match aliases such as macOS /var and /private/var, including not-yet-created children.
    let mut ancestor = result.as_path();
    let mut suffix = Vec::new();
    while !ancestor.exists() {
        let Some(name) = ancestor.file_name() else {
            return Ok(result);
        };
        suffix.push(name.to_os_string());
        let Some(parent) = ancestor.parent() else {
            return Ok(result);
        };
        ancestor = parent;
    }
    let mut resolved = ancestor.canonicalize()?;
    for name in suffix.into_iter().rev() {
        resolved.push(name);
    }
    Ok(resolved)
}

pub fn norm(path: &str) -> String {
    let path = path.replace('\\', "/");
    let path = if path.as_bytes().get(1) == Some(&b':') && !cfg!(windows) {
        path
    } else {
        absolute(Path::new(&path))
            .unwrap_or_else(|_| PathBuf::from(&path))
            .to_string_lossy()
            .to_string()
    };
    path.trim_end_matches('/').to_lowercase()
}

pub fn within(cwd: &str, project: &str) -> bool {
    let cwd = norm(cwd);
    cwd == project || cwd.starts_with(&format!("{project}/"))
}

pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn now() -> chrono::DateTime<chrono::Utc> {
    std::time::SystemTime::now().into()
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<bool> {
    match fs::read(path) {
        Ok(previous) if previous == bytes => return Ok(false),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("cannot read {}", path.display())),
    }
    let parent = path.parent().context("output needs a parent directory")?;
    fs::create_dir_all(parent)?;
    let mut output = tempfile::NamedTempFile::new_in(parent)?;
    if let Ok(metadata) = fs::metadata(path) {
        output.as_file().set_permissions(metadata.permissions())?;
    }
    output.write_all(bytes)?;
    output.as_file().sync_all()?;
    output
        .persist(path)
        .with_context(|| format!("cannot replace {}", path.display()))?;
    Ok(true)
}

pub fn read_text(path: &Path) -> Result<String> {
    let input = fs::File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut bytes = Vec::new();
    if matches!(
        path.extension().and_then(|v| v.to_str()),
        Some("zstd" | "zst")
    ) {
        // Decoder reads all concatenated frames; never split compressed bytes by magic values.
        zstd::stream::read::Decoder::new(input)?.read_to_end(&mut bytes)?;
    } else {
        std::io::BufReader::new(input).read_to_end(&mut bytes)?;
    }
    String::from_utf8(bytes).with_context(|| format!("not UTF-8 text: {}", path.display()))
}

pub fn slice(text: &str, from: usize, to: usize) -> Result<String> {
    if from == 0 || to < from {
        bail!("invalid line range: {from}-{to}");
    }
    let lines: Vec<_> = text.split('\n').collect();
    if from > lines.len() {
        bail!("start line {from} exceeds file length {}", lines.len());
    }
    Ok(lines[from - 1..to.min(lines.len())].join("\n"))
}

pub fn walk(dir: &Path, depth: usize) -> Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    if depth == 0 {
        return Ok(found);
    }
    for entry in fs::read_dir(dir).with_context(|| format!("cannot list {}", dir.display()))? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            found.extend(walk(&entry.path(), depth - 1)?);
        } else if kind.is_file() {
            found.push(entry.path());
        }
    }
    found.sort();
    Ok(found)
}
