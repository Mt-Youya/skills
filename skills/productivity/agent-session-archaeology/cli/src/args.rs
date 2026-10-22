use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Discover local agent sessions and maintain evidence-backed guides offline"
)]
pub struct Cli {
    /// Emit a stable JSON envelope, including errors
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Probe supported/unsupported stores; never reads transcript bodies
    Doctor,
    /// Discover sessions, resolve an exact ID, export or page a session
    Sessions {
        #[command(subcommand)]
        command: Sessions,
    },
    /// Redact credential values in a private export directory
    Redact {
        #[arg(long, default_value = ".agent-sessions")]
        out: PathBuf,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        quiet: bool,
    },
    /// Maintain one incremental gzip evidence archive
    Evidence {
        #[command(subcommand)]
        command: Evidence,
    },
    /// Inventory records and rebuild/check the README index
    Guide {
        #[command(subcommand)]
        command: Guide,
    },
    /// Raw read-only escape hatch: page an exact file by original line number
    Read(Read),
}

#[derive(Args, Clone)]
pub struct Scope {
    #[arg(long, default_value = ".")]
    pub project: PathBuf,
    /// Also search bodies of sessions outside this project; requires broader authorization
    #[arg(long)]
    pub content: bool,
}

#[derive(Subcommand)]
pub enum Sessions {
    /// List ownership-matched sessions, with parse diagnostics and registry differences
    Scan {
        #[command(flatten)]
        scope: Scope,
    },
    /// Resolve an exact source/ID to an original file; fails on ambiguity
    Resolve {
        #[command(flatten)]
        scope: Scope,
        #[arg(long)]
        id: String,
        #[arg(long)]
        source: Option<String>,
    },
    /// Export redacted Markdown; unchanged files are reused
    Export {
        #[command(flatten)]
        scope: Scope,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Include tool data retained by source adapters, not a lossless transcript
        #[arg(long)]
        full: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Page an ownership-confirmed original file without discovery
    Read(Read),
}

#[derive(Args)]
pub struct Read {
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long, default_value_t = 1)]
    pub from: usize,
    #[arg(long)]
    pub to: Option<usize>,
    #[arg(long, default_value_t = 200)]
    pub limit: usize,
}

#[derive(Args)]
pub struct GuidePath {
    #[arg(long, default_value = "docs/avoiding-pitfalls-guide")]
    pub guide: PathBuf,
}

#[derive(Subcommand)]
pub enum Guide {
    /// Read existing IDs, sources, references, topic counts and next free IDs
    Inventory(GuidePath),
    /// Rewrite only the marked README index region
    Index {
        #[command(flatten)]
        path: GuidePath,
        #[arg(long)]
        check: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Check README drift, populated/consecutive chapters and required evidence coverage
    Check {
        #[command(flatten)]
        path: GuidePath,
        #[arg(long, default_value_t = 8.0)]
        budget_mb: f64,
    },
}

#[derive(Subcommand)]
pub enum Evidence {
    /// Append only new excerpts; keep earlier segments for the same ID
    Add {
        #[command(flatten)]
        path: GuidePath,
        #[arg(long)]
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        anchor: Option<String>,
        /// Repeatable FILE:FROM-TO, with 1-based inclusive excerpt-file lines
        #[arg(long)]
        segment: Vec<String>,
        #[arg(long, requires = "lines")]
        session: Option<PathBuf>,
        #[arg(long, requires = "session")]
        lines: Option<String>,
        #[arg(long, default_value_t = 8.0)]
        budget_mb: f64,
        #[arg(long)]
        dry_run: bool,
    },
    List(GuidePath),
    Show {
        #[command(flatten)]
        path: GuidePath,
        #[arg(long)]
        id: String,
    },
    /// Remove named sections only; preview with --dry-run
    Remove {
        #[command(flatten)]
        path: GuidePath,
        #[arg(long, required = true)]
        id: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Write a readable archive copy; defaults to the system temp directory
    Unpack {
        #[command(flatten)]
        path: GuidePath,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    Check {
        #[command(flatten)]
        path: GuidePath,
        #[arg(long, default_value_t = 8.0)]
        budget_mb: f64,
    },
}
