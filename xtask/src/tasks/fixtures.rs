use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

#[allow(clippy::module_inception)]
#[path = "../../../tests/fixtures/mod.rs"]
mod fixtures;

use fixtures::mkrepo::RepoBuilder;
use fixtures::scenarios;

struct ScenarioSpec {
    name: &'static str,
    runner: fn(&mut RepoBuilder) -> Result<Vec<git2::Oid>, git2::Error>,
}

const ALL_SCENARIOS: &[ScenarioSpec] = &[
    ScenarioSpec {
        name: "linear_history",
        runner: scenarios::scenario_linear_history,
    },
    ScenarioSpec {
        name: "file_rename",
        runner: scenarios::scenario_file_rename,
    },
    ScenarioSpec {
        name: "cross_file_move",
        runner: scenarios::scenario_cross_file_move,
    },
    ScenarioSpec {
        name: "squash_merge",
        runner: scenarios::scenario_squash_merge,
    },
    ScenarioSpec {
        name: "evil_merge",
        runner: scenarios::scenario_evil_merge,
    },
    ScenarioSpec {
        name: "broken_syntax",
        runner: scenarios::scenario_broken_syntax,
    },
];

/// Runs scenario generation, optionally exporting to a directory for manual inspection.
pub fn run_fixtures(dump_dir: Option<&Path>, scenario_filter: Option<&str>) -> Result<()> {
    let scenarios_to_run: Vec<&ScenarioSpec> = if let Some(filter) = scenario_filter {
        let normalized = filter.to_lowercase().replace('-', "_");
        let matched: Vec<&ScenarioSpec> = ALL_SCENARIOS
            .iter()
            .filter(|s| s.name.contains(&normalized) || normalized == "all")
            .collect();
        if matched.is_empty() {
            bail!(
                "Unknown scenario '{filter}'. Available scenarios: linear_history, file_rename, cross_file_move, squash_merge, evil_merge, broken_syntax"
            );
        }
        matched
    } else {
        ALL_SCENARIOS.iter().collect()
    };

    println!("\n=== GitCtx Test Fixtures Generator ===");
    if let Some(dir) = dump_dir {
        println!("Export Target Directory: {}", dir.display());
        std::fs::create_dir_all(dir)
            .with_context(|| format!("Failed to create dump directory at {}", dir.display()))?;
    } else {
        println!("Export Target: In-Memory / Temporary (Pass --dump-dir <path> to export)");
    }
    println!("{:-<80}", "");
    println!(
        "{:<20} | {:<8} | {:<40} | Path",
        "Scenario", "Commits", "HEAD OID"
    );
    println!("{:-<80}", "");

    for spec in scenarios_to_run {
        let mut builder = if let Some(base_dir) = dump_dir {
            let target_path: PathBuf = base_dir.join(spec.name);
            if target_path.exists() {
                std::fs::remove_dir_all(&target_path).with_context(|| {
                    format!(
                        "Failed to clean existing directory at {}",
                        target_path.display()
                    )
                })?;
            }
            RepoBuilder::new_in(&target_path)
                .with_context(|| format!("Failed to create repo in {}", target_path.display()))?
        } else {
            RepoBuilder::new().context("Failed to create temporary repo")?
        };

        let oids = (spec.runner)(&mut builder)
            .map_err(|e| anyhow::anyhow!("Scenario '{}' failed: {e}", spec.name))?;

        let head_oid = builder
            .head_oid()
            .map_or_else(|_| "UNKNOWN".to_string(), |oid| oid.to_string());

        let display_path = if dump_dir.is_some() {
            builder.path().display().to_string()
        } else {
            "<tempdir>".to_string()
        };

        println!(
            "{:<20} | {:<8} | {:<40} | {display_path}",
            spec.name,
            oids.len(),
            head_oid
        );
    }

    println!("{:-<80}\n", "");
    println!("Fixtures generation completed successfully.");
    Ok(())
}
