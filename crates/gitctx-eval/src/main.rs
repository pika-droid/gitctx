#![forbid(unsafe_code)]

//! `gitctx-eval` CLI binary for running synthetic mutation generation,
//! manifest inspections, benchmark evaluations, and threshold tuning.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use gitctx_eval::mutator::{LineageGroundTruthManifest, MutationEngine};
use gitctx_eval::oracle::{GitLogOracleComparator, MutationBenchmarkRunner, ThresholdTuner};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gitctx-eval",
    about = "Evaluation, benchmark, and synthetic mutation engine for gitctx"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate synthetic mutation repositories and lineage ground-truth manifests
    Mutate {
        /// Mutation class to apply (M1..M10 or 'all')
        #[arg(long, default_value = "all")]
        class: String,

        /// Sequential mutation chain depth (applied when single class selected)
        #[arg(long, default_value = "1")]
        chain_depth: usize,

        /// Directory to output the generated repository and lineage_ground_truth.json
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Inspect and print a summary of a lineage ground truth manifest file
    InspectManifest {
        /// Path to lineage_ground_truth.json
        #[arg(long)]
        manifest: PathBuf,
    },

    /// Run synthetic mutation benchmark evaluation across M1-M10 and chain depths
    BenchLineage {
        /// Output format (terminal, markdown, or json)
        #[arg(long, default_value = "terminal")]
        format: String,
    },

    /// Run parameter sweep to calibrate optimal match thresholds
    TuneThresholds {
        /// Path to save the calibrated thresholds config (default: eval/lineage_thresholds.json)
        #[arg(long, default_value = "eval/lineage_thresholds.json")]
        out: PathBuf,
    },

    /// Compare gitctx lineage tracking against git log -L heuristic oracle
    EvalOracle {
        /// Path to repository to evaluate
        #[arg(long)]
        repo: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Mutate {
            class,
            chain_depth,
            out,
        } => run_mutate(&class, chain_depth, out.as_deref()),
        Commands::InspectManifest { manifest } => run_inspect_manifest(&manifest),
        Commands::BenchLineage { format } => run_bench_lineage(&format),
        Commands::TuneThresholds { out } => run_tune_thresholds(&out),
        Commands::EvalOracle { repo } => run_eval_oracle(&repo),
    }
}

fn run_bench_lineage(format: &str) -> Result<()> {
    let runner = MutationBenchmarkRunner::new();
    let report = runner
        .run_benchmark_suite()
        .context("Failed running synthetic mutation benchmark suite")?;

    match format.to_lowercase().as_str() {
        "markdown" | "md" => {
            println!("{}", runner.format_markdown_table(&report));
        }
        "json" => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        _ => {
            print!("{}", runner.format_terminal_table(&report));
        }
    }

    Ok(())
}

fn run_tune_thresholds(out_path: &std::path::Path) -> Result<()> {
    println!("\n=== GitCtx Lineage Threshold Calibration Sweep ===");
    println!("Output Target: {}", out_path.display());

    let tuner = ThresholdTuner::new();
    let config = tuner.sweep().context("Failed executing threshold sweep")?;

    println!("\nOptimal Calibrated Thresholds:");
    println!("- tau_same:                 {:.2}", config.tau_same);
    println!("- tau_cross:                {:.2}", config.tau_cross);
    println!("- tau_decoy:                {:.2}", config.tau_decoy);
    println!("- Objective F1 Score:       {:.3}", config.overall_f1);
    println!(
        "- M8 Decoy Rejection Rate:  {:.1}%",
        config.m8_decoy_rejection_rate * 100.0
    );
    println!(
        "- Chain Recovery (D=5):     {:.1}%",
        config.chain_recovery_depth_5 * 100.0
    );
    println!(
        "- Grid Points Evaluated:    {}",
        config.grid_points_evaluated
    );

    ThresholdTuner::save_config(&config, out_path)?;
    println!("\nSuccessfully saved: {}\n", out_path.display());
    Ok(())
}

fn run_eval_oracle(repo_path: &std::path::Path) -> Result<()> {
    println!("\n=== GitCtx vs git log -L Oracle Comparator ===");
    println!("Repository: {}", repo_path.display());

    let sample_funcs = vec![
        ("calculate_sum".to_string(), "math.py".to_string(), false),
        ("process_data".to_string(), "data.py".to_string(), true),
    ];

    let report = GitLogOracleComparator::evaluate_repository(repo_path, &sample_funcs)
        .context("Failed running oracle comparison")?;

    let comparator = GitLogOracleComparator;
    print!("{}", comparator.format_report(&report));
    Ok(())
}

fn run_mutate(class: &str, chain_depth: usize, out_dir: Option<&std::path::Path>) -> Result<()> {
    println!("\n=== GitCtx Synthetic Mutation Generator ===");
    println!("Class Filter: {class}");
    println!("Chain Depth:  {chain_depth}");
    if let Some(dir) = out_dir {
        println!("Output Target: {}", dir.display());
    } else {
        println!("Output Target: In-Memory / Temporary (Pass --out <dir> to export)");
    }

    let (builder, manifest) = MutationEngine::generate_and_save(out_dir, Some(class), chain_depth)
        .map_err(|e| anyhow::anyhow!("Mutation generation failed: {e}"))?;

    println!("{:-<90}", "");
    println!(
        "{:<10} | {:<25} | {:<12} | {:<20} -> {:<20}",
        "Commit", "Mutation Class", "Relation", "Source", "Target"
    );
    println!("{:-<90}", "");

    for link in &manifest.links {
        let short_sha = if link.commit_sha.len() >= 8 {
            &link.commit_sha[..8]
        } else {
            &link.commit_sha
        };

        println!(
            "{:<10} | {:<25} | {:<12} | {:<20} -> {:<20}",
            short_sha,
            format!("{:?}", link.mutation_class),
            format!("{}", link.relation),
            link.source.qualname,
            link.target.qualname
        );
    }

    println!("{:-<90}", "");
    println!(
        "Total Commits: {} | Ground Truth Links: {}",
        builder.commit_count(),
        manifest.links.len()
    );
    if let Some(dir) = out_dir {
        println!(
            "Saved Manifest: {}",
            dir.join("lineage_ground_truth.json").display()
        );
    }
    println!("{:-<90}\n", "");

    Ok(())
}

fn run_inspect_manifest(path: &std::path::Path) -> Result<()> {
    println!("\n=== Inspect Lineage Ground Truth Manifest ===");
    println!("Manifest Path: {}", path.display());

    let manifest = LineageGroundTruthManifest::load_from_file(path)
        .with_context(|| format!("Failed to read manifest at {}", path.display()))?;

    println!("Schema Version: {}", manifest.schema_version);
    println!("Created At:     {}", manifest.created_at);
    println!("Chain Depth:    {}", manifest.chain_depth);
    println!("Links Count:    {}", manifest.links.len());
    println!("{:-<90}", "");
    println!(
        "{:<10} | {:<25} | {:<12} | {:<20} -> {:<20}",
        "Commit", "Mutation Class", "Relation", "Source", "Target"
    );
    println!("{:-<90}", "");

    for link in &manifest.links {
        let short_sha = if link.commit_sha.len() >= 8 {
            &link.commit_sha[..8]
        } else {
            &link.commit_sha
        };

        println!(
            "{:<10} | {:<25} | {:<12} | {:<20} -> {:<20}",
            short_sha,
            format!("{:?}", link.mutation_class),
            format!("{}", link.relation),
            link.source.qualname,
            link.target.qualname
        );
    }
    println!("{:-<90}\n", "");

    Ok(())
}
