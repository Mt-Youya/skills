use super::args::{self, Cli, Command, Evidence, Guide, Sessions};
use crate::{
    guide::{self, evidence},
    sessions::{discovery, environment, export, read},
    storage::{files, redact},
};
use anyhow::Result;
use serde_json::{json, Value};

fn page(args: &args::Read) -> Result<Value> {
    read::page(&args.file, args.from, args.to, args.limit)
}

pub(super) fn run(cli: Cli) -> Result<Value> {
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
            Sessions::Scan { scope, since } => Ok(discovery::scan(
                &environment::Environment::load()?,
                &scope.project,
                scope.content,
                since.as_deref(),
            )?
            .report()),
            Sessions::Resolve { scope, id, source } => discovery::resolve(
                &environment::Environment::load()?,
                &scope.project,
                scope.content,
                &id,
                source.as_deref(),
            ),
            Sessions::Export {
                scope,
                out,
                full,
                dry_run,
            } => {
                let discovery = discovery::discover(
                    &environment::Environment::load()?,
                    &scope.project,
                    scope.content,
                )?;
                let out = out.unwrap_or_else(|| discovery.project.join(".agent-sessions"));
                export::export(&discovery, &out, full, dry_run)
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
