#![forbid(unsafe_code)]

pub mod commands;
pub mod envelope;
pub mod format;

use clap::{Parser, ValueEnum};
use commands::Commands;
use envelope::ResponseEnvelope;

#[derive(Parser, Debug)]
#[command(
    name = "gitctx",
    about = "Offline Git-native codebase intelligence engine",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = Format::Table, global = true)]
    pub format: Format,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Json,
    Table,
}

pub fn run(cli: Cli) -> anyhow::Result<()> {
    let format = cli.format;
    match cli.command {
        Some(Commands::Index { repo_path }) => {
            commands::index::run(&repo_path, format)?;
        }
        Some(Commands::Search { query }) => {
            commands::search::run(&query, format)?;
        }
        Some(Commands::Evolution { identifier }) => {
            commands::evolution::run(&identifier, format)?;
        }
        Some(Commands::Show { hash }) => {
            commands::show::run(&hash, format)?;
        }
        Some(Commands::Impact { target }) => {
            commands::impact::run(target.as_deref(), format)?;
        }
        Some(Commands::Blame { file_path }) => {
            commands::blame::run(&file_path, format)?;
        }
        Some(Commands::Doctor) => {
            commands::doctor::run(format)?;
        }
        Some(Commands::Init) => {
            commands::init::run(format)?;
        }
        None => {
            if format == Format::Json {
                let env = ResponseEnvelope::success("gitctx CLI initialized");
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("gitctx CLI initialized. Use --help for usage instructions.");
            }
        }
    }
    Ok(())
}
