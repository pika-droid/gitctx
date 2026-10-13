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
        /// Also run dependency checks (cargo machete, deny, audit)
        #[arg(long)]
        deps: bool,
    },
    /// Run dependency health checks (cargo machete, cargo deny, cargo audit)
    CheckDeps {
        /// Treat missing tools as warnings rather than hard failures
        #[arg(long)]
        allow_missing: bool,
    },
    /// Run security and license audit (cargo audit, cargo deny)
    Audit {
        /// Treat missing tools as warnings rather than hard failures
        #[arg(long)]
        allow_missing: bool,
    },
    /// Run semantic versioning verification (cargo semver-checks)
    Semver {
        /// Baseline git revision (e.g. main or origin/main)
        #[arg(long)]
        baseline_rev: Option<String>,
        /// Specific package to check
        #[arg(long)]
        package: Option<String>,
        /// Treat missing tool as warning rather than hard failure
        #[arg(long)]
        allow_missing: bool,
    },
    /// Run test suite using nextest if available, or cargo test
    Test {
        /// Force using standard cargo test instead of nextest
        #[arg(long)]
        cargo_test: bool,
        /// Optional test name filter
        #[arg(long)]
        filter: Option<String>,
    },
    /// Run code coverage using cargo-llvm-cov
    Coverage {
        /// Generate HTML report in target/llvm-cov/html
        #[arg(long)]
        html: bool,
        /// Generate LCOV report at lcov.info
        #[arg(long)]
        lcov: bool,
        /// Treat missing tool as warning rather than hard failure
        #[arg(long)]
        allow_missing: bool,
    },
    /// Run Miri undefined-behavior verification on core algorithms
    Miri {
        /// Specific package to check (default: gitctx-core)
        #[arg(long)]
        package: Option<String>,
        /// Treat missing tool as warning rather than hard failure
        #[arg(long)]
        allow_missing: bool,
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
        /// Run all evaluation stages (ablation grid then evaluation report)
        #[arg(long)]
        all: bool,
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
    DownloadModels {
        /// Optional model identifier (e.g. BAAI/bge-small-en-v1.5)
        #[arg(long)]
        model: Option<String>,
        /// Destination directory for model weights
        #[arg(long)]
        dest: Option<PathBuf>,
    },
    /// Download cross-platform ONNX Runtime shared library
    DownloadOrtRuntime,
    /// Clone or update pinned evaluation corpus into `eval/corpus/` per Invariant I6
    CloneCorpus {
        /// Optional path to corpus.lock.toml file
        #[arg(long)]
        lockfile: Option<PathBuf>,
        /// Optional destination directory for corpus clones
        #[arg(long)]
        target_dir: Option<PathBuf>,
    },
    /// Profile candidate repositories and generate selection docs / lockfile
    ProfileCorpus {
        /// Optional candidate repository list or comma-separated repo IDs
        #[arg(long)]
        candidates: Option<String>,
        /// Run profile strictly against local repositories without network calls
        #[arg(long)]
        offline: bool,
    },
    /// Harvest issue-closing pairs (Set A) and revert pairs into canonical datasets
    Harvest {
        /// Optional specific repository identifier to harvest
        #[arg(long)]
        repo: Option<String>,
        /// Run harvest strictly offline using local git history and cache
        #[arg(long)]
        offline: bool,
    },
    /// Run interactive precision audit CLI and calculate Wilson Score CI
    AuditHarvester {
        /// Number of sample pairs to audit (default: 30)
        #[arg(long, default_value = "30")]
        samples: usize,
        /// Optional pre-recorded batch judgments JSON file
        #[arg(long)]
        batch_file: Option<PathBuf>,
        /// Use synthetic ground-truth dataset
        #[arg(long)]
        synthetic: bool,
        /// Run automated non-interactive audit
        #[arg(long)]
        non_interactive: bool,
    },
    /// Verify cryptographic integrity of ground truth datasets against MANIFEST.toml
    VerifyGroundTruth,
    /// Run synthetic mutation lineage accuracy benchmark evaluation
    EvalLineage {
        /// Output format (terminal, markdown, or json)
        #[arg(long, default_value = "terminal")]
        format: Option<String>,
    },
    /// Run threshold calibration tuning sweep
    TuneLineage {
        /// Destination path for calibrated lineage_thresholds.json
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Evaluate function lineage against git log -L heuristic oracle
    EvalOracle {
        /// Path to repository to evaluate
        #[arg(long)]
        repo: PathBuf,
    },
    /// Run performance benchmark suite
    Bench,
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
    /// Run synthetic lineage benchmark evaluation
    BenchLineage {
        /// Output format (terminal, markdown, or json)
        #[arg(long, default_value = "terminal")]
        format: Option<String>,
    },
    /// Run parameter sweep to tune match thresholds
    TuneThresholds {
        /// Path to save calibrated thresholds config
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Compare lineage tracking against git log -L oracle
    EvalOracle {
        /// Path to repository to evaluate
        #[arg(long)]
        repo: PathBuf,
    },
    /// Run 5x5 retrieval ablation grid evaluation across D1-D5 and R1-R5
    Ablation {
        /// Directory containing eval artifacts (default: eval/)
        #[arg(long)]
        eval_dir: Option<PathBuf>,
    },
    /// Generate comprehensive evaluation report (eval/REPORT.md)
    Report {
        /// Directory containing eval artifacts (default: eval/)
        #[arg(long)]
        eval_dir: Option<PathBuf>,
        /// Output path for the report (default: eval/REPORT.md)
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Lint { fix, deps } => tasks::lint::run_lint(fix, deps),
        Commands::CheckDeps { allow_missing } => tasks::check_deps::run_check_deps(allow_missing),
        Commands::Audit { allow_missing } => tasks::check_deps::run_audit(allow_missing),
        Commands::Semver {
            baseline_rev,
            package,
            allow_missing,
        } => tasks::semver::run_semver(baseline_rev.as_deref(), package.as_deref(), allow_missing),
        Commands::Test { cargo_test, filter } => {
            tasks::test::run_test(cargo_test, filter.as_deref())
        }
        Commands::Coverage {
            html,
            lcov,
            allow_missing,
        } => tasks::coverage::run_coverage(html, lcov, allow_missing),
        Commands::Miri {
            package,
            allow_missing,
        } => tasks::miri::run_miri(package.as_deref(), allow_missing),
        Commands::CloneReferences { repo } => {
            tasks::clone_references::run_clone_references(repo.as_deref())
        }
        Commands::TestOffline => tasks::test_offline::run_test_offline(),
        Commands::TestInvariants => tasks::test_invariants::run_test_invariants(),
        Commands::Fixtures { dump_dir, scenario } => {
            tasks::fixtures::run_fixtures(dump_dir.as_deref(), scenario.as_deref())
        }
        Commands::Eval {
            subcommand,
            all,
            class,
            chain_depth,
            out,
        } => {
            if all {
                tasks::eval::run_eval_ablation(None)?;
                tasks::eval::run_eval_report(None, None)?;
                Ok(())
            } else {
                match subcommand {
                    Some(EvalSubcommands::Mutate {
                        class: sub_class,
                        chain_depth: sub_depth,
                        out: sub_out,
                    }) => tasks::eval::run_eval_mutate(sub_class.as_deref(), sub_depth, sub_out.as_deref()),
                    Some(EvalSubcommands::BenchLineage { format }) => {
                        tasks::eval::run_eval_bench_lineage(format.as_deref())
                    }
                    Some(EvalSubcommands::TuneThresholds { out: sub_out }) => {
                        tasks::eval::run_eval_tune_thresholds(sub_out.as_deref())
                    }
                    Some(EvalSubcommands::EvalOracle { repo }) => tasks::eval::run_eval_oracle(&repo),
                    Some(EvalSubcommands::Ablation { eval_dir }) => {
                        tasks::eval::run_eval_ablation(eval_dir.as_deref())
                    }
                    Some(EvalSubcommands::Report { eval_dir, output }) => {
                        tasks::eval::run_eval_report(eval_dir.as_deref(), output.as_deref())
                    }
                    None => tasks::eval::run_eval_mutate(class.as_deref(), chain_depth, out.as_deref()),
                }
            }
        }
        Commands::DownloadModels { model, dest } => {
            tasks::download_models::run_download_models(model.as_deref(), dest.as_deref())
        }
        Commands::DownloadOrtRuntime => tasks::download_models::run_download_ort_runtime(),
        Commands::CloneCorpus {
            lockfile,
            target_dir,
        } => tasks::clone_corpus::run_clone_corpus(lockfile.as_deref(), target_dir.as_deref()),
        Commands::ProfileCorpus {
            candidates,
            offline,
        } => tasks::profile_corpus::run_profile_corpus(candidates.as_deref(), offline),
        Commands::Harvest { repo, offline } => {
            tasks::harvest::run_harvest(repo.as_deref(), offline)
        }
        Commands::AuditHarvester {
            samples,
            batch_file,
            synthetic,
            non_interactive,
        } => tasks::audit_harvester::run_audit_harvester(
            samples,
            batch_file.as_deref(),
            synthetic,
            non_interactive,
        ),
        Commands::VerifyGroundTruth => tasks::verify_ground_truth::run_verify_ground_truth(),
        Commands::EvalLineage { format } => tasks::eval::run_eval_bench_lineage(format.as_deref()),
        Commands::TuneLineage { out } => tasks::eval::run_eval_tune_thresholds(out.as_deref()),
        Commands::EvalOracle { repo } => tasks::eval::run_eval_oracle(&repo),
        Commands::Bench => tasks::bench::run_bench(),
    }
}
