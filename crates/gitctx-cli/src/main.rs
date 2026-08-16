#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "gitctx",
    about = "Offline Git-native codebase intelligence engine",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Index a local Git repository
    Index {
        /// Path to repository root
        #[arg(default_value = ".")]
        repo_path: String,
    },
    /// Query the indexed codebase
    Query {
        /// Natural language or identifier query
        query: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Index { repo_path }) => {
            println!("Indexing repository at: {repo_path}");
        }
        Some(Commands::Query { query }) => {
            println!("Querying repository for: {query}");
        }
        None => {
            println!("gitctx CLI initialized. Use --help for usage instructions.");
        }
    }
    Ok(())
}
