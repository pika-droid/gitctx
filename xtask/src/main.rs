use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod tasks;
mod util;

#[derive(Parser, Debug)]
#[command(
    name = "xtask",
    about = "Internal development task automation runner for gitctx"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run code formatting (`cargo fmt`) and linter (`cargo clippy`) checks
    Lint {
        /// Automatically apply suggested fixes
        #[arg(long)]
        fix: bool,
    },
    /// Clone or update vetted reference repositories into `reference_repos/`
    CloneReferences {
        /// Optional specific reference repository name to clone
        #[arg(long)]
        repo: Option<String>,
    },
    /// Run offline invariant test harness
    TestOffline,
    /// Run full invariant test verification suite
    TestInvariants,
    /// Generate or dump test repository fixtures
    Fixtures {
        /// Directory to export generated test fixtures
        #[arg(long)]
        dump_dir: Option<PathBuf>,
        /// Optional specific scenario filter (linear, rename, move, squash, evil_merge, broken_syntax)
        #[arg(long)]
        scenario: Option<String>,
    },
    /// Run evaluation and synthetic mutation engine
    Eval {
        #[command(subcommand)]
        subcommand: Option<EvalSubcommands>,
        /// Optional mutation class or evaluation subcommand
        #[arg(long)]
        class: Option<String>,
        /// Sequential mutation chain depth
        #[arg(long, default_value = "1")]
        chain_depth: usize,
        /// Optional output directory for generated repository and manifest
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Download local embedding models and tokenizers
    DownloadModels,
}

#[derive(Subcommand, Debug)]
enum EvalSubcommands {
    /// Run synthetic mutation generation
    Mutate {
        /// Mutation class to apply (M1..M10 or 'all')
        #[arg(long)]
        class: Option<String>,
        /// Sequential mutation chain depth
        #[arg(long, default_value = "1")]
        chain_depth: usize,
        /// Directory to output the generated repository and lineage_ground_truth.json
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Lint { fix } => tasks::lint::run_lint(fix),
        Commands::CloneReferences { repo } => {
            tasks::clone_references::run_clone_references(repo.as_deref())
        }
        Commands::TestOffline => tasks::stub_command("test-offline"),
        Commands::TestInvariants => tasks::stub_command("test-invariants"),
        Commands::Fixtures { dump_dir, scenario } => {
            tasks::fixtures::run_fixtures(dump_dir.as_deref(), scenario.as_deref())
        }
        Commands::Eval {
            subcommand,
            class,
            chain_depth,
            out,
        } => match subcommand {
            Some(EvalSubcommands::Mutate {
                class: sub_class,
                chain_depth: sub_depth,
                out: sub_out,
            }) => tasks::eval::run_eval_mutate(
                sub_class.as_deref(),
                sub_depth,
                sub_out.as_deref(),
            ),
            None => tasks::eval::run_eval_mutate(class.as_deref(), chain_depth, out.as_deref()),
        },
        Commands::DownloadModels => tasks::stub_command("download-models"),
    }
}
