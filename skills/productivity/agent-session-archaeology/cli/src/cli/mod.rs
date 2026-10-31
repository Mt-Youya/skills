mod args;
mod commands;

use crate::storage::redact;
use args::Cli;
use clap::Parser;
use serde_json::{json, Value};
use std::io::Write;

fn emit(value: &Value) -> std::io::Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    stdout.write_all(b"\n")
}

pub(crate) fn main() -> std::process::ExitCode {
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
    let (value, code) = match commands::run(cli) {
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
