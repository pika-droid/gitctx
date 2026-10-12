use crate::envelope::ResponseEnvelope;
use crate::format::{format_impact_global, format_impact_single};
use crate::Format;
use anyhow::{Context, Result};
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use gitctx_query::cochange::{ExtractionStats, ImpactService};
use std::path::Path;
use std::time::Instant;

pub fn run(target: Option<&str>, format: Format) -> Result<()> {
    let start_time = Instant::now();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let db_path = current_dir.join(".gitctx").join("index.db");

    if !db_path.exists() {
        anyhow::bail!(
            "Index database not found at {}. Have you run 'gitctx index'?",
            db_path.display()
        );
    }

    let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "BGESmallENV15", "code", 500, 1);

    let storage =
        StorageEngine::open(&db_path, &config).context("Failed to open storage engine")?;

    let impact_service = ImpactService::new(&storage);

    if let Some(t) = target {
        let impact = impact_service
            .analyze(t, 10)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        if format == Format::Json {
            let env = ResponseEnvelope::success(serde_json::json!({
                "status": "success",
                "impact": impact,
                "duration_ms": start_time.elapsed().as_millis(),
            }));
            println!("{}", serde_json::to_string_pretty(&env)?);
        } else {
            format_impact_single(t, &impact);
        }
    } else {
        // Global analysis
        let global = impact_service
            .global_metrics(10)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let stats = ExtractionStats {
            total_commits: 0,
            qualifying_commits: 0,
            size_filtered: 0,
            bot_filtered: 0,
            total_file_pairs: global.top_file_couplings.len(),
            total_function_pairs: global.top_function_couplings.len(),
        };

        if format == Format::Json {
            let env = ResponseEnvelope::success(serde_json::json!({
                "status": "success",
                "top_file_couplings": global.top_file_couplings,
                "top_function_couplings": global.top_function_couplings,
                "duration_ms": start_time.elapsed().as_millis(),
            }));
            println!("{}", serde_json::to_string_pretty(&env)?);
        } else {
            format_impact_global(
                &stats,
                &global.top_file_couplings,
                &global.top_function_couplings,
            );
        }
    }

    Ok(())
}
