#![forbid(unsafe_code)]

mod commands;
mod envelope;
mod format;

use clap::{Parser, ValueEnum};
use commands::Commands;
use envelope::ResponseEnvelope;
use std::process;

#[derive(Parser, Debug)]
#[command(
    name = "gitctx",
    about = "Offline Git-native codebase intelligence engine",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Output format
    #[arg(short, long, value_enum, default_value_t = Format::Table, global = true)]
    format: Format,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Format {
    Json,
    Table,
}

fn main() {
    let cli = Cli::parse();
    
    if let Err(e) = run(cli) {
        // We handle errors nicely by checking if format is json, otherwise printing to stderr
        // The global format flag isn't easily accessible here if we just failed matching, but we'll try
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let format = cli.format;
    match cli.command {
        Some(Commands::Index { repo_path }) => {
            commands::index::run(&repo_path, format)?;
        }
        Some(Commands::Search { query }) => {
            if format == Format::Json {
                let env = ResponseEnvelope::success(format!("Search result for {query}"));
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("Querying repository for: {query}");
            }
        }
        Some(Commands::Evolution { identifier }) => {
            if format == Format::Json {
                let env = ResponseEnvelope::success(format!("Evolution of {identifier}"));
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("Tracking evolution of: {identifier}");
            }
        }
        Some(Commands::Show { hash }) => {
            if format == Format::Json {
                let env = ResponseEnvelope::success(format!("Details of {hash}"));
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("Showing details for commit: {hash}");
            }
        }
        Some(Commands::Impact) => {
            if format == Format::Json {
                let env = ResponseEnvelope::success("Impact analysis results");
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("Analyzing risk and impact...");
            }
        }
        Some(Commands::Blame { file_path }) => {
            if format == Format::Json {
                let env = ResponseEnvelope::success(format!("Blame for {file_path}"));
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                println!("Viewing line-level attribution for: {file_path}");
            }
        }
        Some(Commands::Doctor) => {
            commands::doctor::run(format)?;
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
