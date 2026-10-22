mod adapters;
mod args;
mod environment;
mod evidence;
mod files;
mod guide;
mod redact;
mod session;
mod sources;

use anyhow::{bail, Result};
use args::{Cli, Command, Evidence, Guide, Sessions};
use clap::Parser;
use serde_json::{json, Value};
use std::io::Write;

fn page(args: &args::Read) -> Result<Value> {
    session::page(&args.file, args.from, args.to, args.limit)
}

fn run(cli: Cli) -> Result<Value> {
    match cli.command {
        Command::Doctor => Ok(environment::Environment::load()?.doctor()),
        Command::Read(args) => page(&args),
        Command::Redact {
            out,
            check,
            dry_run,
            quiet: _,
        } => {
            let mut result = redact::directory(&files::absolute(&out)?, check || dry_run)?;
            result["check_failed"] = json!(check && result["touched"].as_u64().unwrap_or(0) > 0);
            Ok(result)
        }
        Command::Sessions { command } => match command {
            Sessions::Read(args) => page(&args),
            Sessions::Scan { scope } => Ok(sources::discover(
                &environment::Environment::load()?,
                &scope.project,
                scope.content,
            )?
            .report()),
            Sessions::Resolve { scope, id, source } => {
                let discovery = sources::discover(
                    &environment::Environment::load()?,
                    &scope.project,
                    scope.content,
                )?;
                let found: Vec<_> = discovery
                    .sessions
                    .iter()
                    .filter(|s| s.id == id && source.as_ref().is_none_or(|src| src == &s.source))
                    .collect();
                if found.len() != 1 {
                    bail!("expected one source/session match for {id}, found {}; use --source to disambiguate",found.len());
                }
                Ok(found[0].row())
            }
            Sessions::Export {
                scope,
                out,
                full,
                dry_run,
            } => {
                let discovery = sources::discover(
                    &environment::Environment::load()?,
                    &scope.project,
                    scope.content,
                )?;
                let out = out.unwrap_or_else(|| discovery.project.join(".agent-sessions"));
                sources::export(&discovery, &out, full, dry_run)
            }
        },
        Command::Guide { command } => match command {
            Guide::Inventory(path) => guide::inventory(&files::absolute(&path.guide)?),
            Guide::Index {
                path,
                check,
                dry_run,
            } => guide::index(&files::absolute(&path.guide)?, check, dry_run),
            Guide::Check { path, budget_mb } => {
                let guide = files::absolute(&path.guide)?;
                guide::check_chapters(&guide)?;
                guide::index(&guide, true, false)?;
                evidence::check(&guide, budget_mb)
            }
        },
        Command::Evidence { command } => match command {
            Evidence::Add {
                path,
                id,
                title,
                anchor,
                mut segment,
                session,
                lines,
                budget_mb,
                dry_run,
            } => {
                if let Some(file) = session {
                    segment.push(format!("{}:{}", file.display(), lines.unwrap()));
                }
                evidence::add(evidence::Add {
                    guide: &files::absolute(&path.guide)?,
                    id: &id,
                    title: title.as_deref(),
                    anchor: anchor.as_deref(),
                    requested: segment,
                    budget: budget_mb,
                    dry_run,
                })
            }
            Evidence::List(path) => evidence::list(&files::absolute(&path.guide)?),
            Evidence::Show { path, id } => {
                Ok(json!({"id":id,"text":evidence::show(&files::absolute(&path.guide)?,&id)?}))
            }
            Evidence::Remove { path, id, dry_run } => {
                evidence::remove(&files::absolute(&path.guide)?, &id, dry_run)
            }
            Evidence::Unpack { path, out, dry_run } => {
                evidence::unpack(&files::absolute(&path.guide)?, out.as_deref(), dry_run)
            }
            Evidence::Check { path, budget_mb } => {
                evidence::check(&files::absolute(&path.guide)?, budget_mb)
            }
        },
    }
}

fn emit(value: &Value) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    stdout.write_all(b"\n")
}

fn main() -> std::process::ExitCode {
    let json_requested = std::env::args_os().any(|arg| arg == "--json");
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = error.print();
                return std::process::ExitCode::SUCCESS;
            }
            let message = redact::text(&error.to_string());
            if json_requested {
                let _ = emit(&json!({"ok":false,"error":{"code":2,"message":message}}));
            } else {
                eprintln!("{message}");
            }
            return std::process::ExitCode::from(2);
        }
    };
    let json_mode = cli.json;
    let (value, code) = match run(cli) {
        Ok(data) => {
            let code = if data["over_budget"] == true {
                2
            } else if data["check_failed"] == true || data["incomplete"] == true {
                1
            } else {
                0
            };
            let value = if json_mode {
                if code == 0 {
                    json!({"ok":true,"data":data})
                } else {
                    json!({"ok":false,"data":data,"error":{"code":code,"message":if code==2{"guide exceeds --budget-mb"}else if data["incomplete"]==true{"session inventory/export is incomplete; inspect diagnostics and registry differences"}else{"credential values remain in exports"}}})
                }
            } else {
                data
            };
            (value, code)
        }
        Err(error) => {
            let message = redact::text(&format!("{error:#}"));
            if !json_mode {
                eprintln!("Error: {message}");
                return std::process::ExitCode::FAILURE;
            }
            (json!({"ok":false,"error":{"code":1,"message":message}}), 1)
        }
    };
    let written = if !json_mode && value["text"].is_string() {
        writeln!(
            std::io::stdout().lock(),
            "{}",
            value["text"].as_str().unwrap()
        )
    } else {
        // Stable metadata IDs remain intact; transcript bodies are redacted at their read boundary.
        emit(&value)
    };
    match written {
        Ok(()) => std::process::ExitCode::from(code),
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot write output: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
