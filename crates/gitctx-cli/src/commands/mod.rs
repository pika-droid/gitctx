pub mod doctor;
pub mod index;

use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index a local Git repository
    Index {
        /// Path to repository root
        #[arg(default_value = ".")]
        repo_path: String,
    },
    /// Query the indexed codebase
    Search {
        /// Natural language or identifier query
        query: String,
    },
    /// Track the evolution of a function over time
    Evolution {
        /// Identifier of the function to track
        identifier: String,
    },
    /// Show detailed information about a specific commit
    Show {
        /// Commit hash to show
        hash: String,
    },
    /// Analyze risk and impact for recent commits
    Impact,
    /// View line-level attribution for a file
    Blame {
        /// File path to blame
        file_path: String,
    },
    /// Run diagnostic checks on the index and environment
    Doctor,
}
