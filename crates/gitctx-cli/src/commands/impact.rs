use crate::envelope::ResponseEnvelope;
use crate::format::{format_impact_global, format_impact_single};
use crate::Format;
use anyhow::{Context, Result};
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use gitctx_query::cochange::{
    CoChangeConfig, CoChangeExtractor, CoChangeGraph, EntityType, ImpactAnalyzer, MetricsComputer,
};
use std::path::Path;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let now_epoch = i64::try_from(secs).unwrap_or(i64::MAX);
    let cochange_config = CoChangeConfig::with_reference_epoch(now_epoch);

    let extractor = CoChangeExtractor::new(&storage, &cochange_config);
    let (file_pairs, func_pairs, stats) = extractor.extract().map_err(|e| anyhow::anyhow!(e))?;

    let file_metrics =
        MetricsComputer::compute(&file_pairs, stats.qualifying_commits, &cochange_config);
    let func_metrics =
        MetricsComputer::compute(&func_pairs, stats.qualifying_commits, &cochange_config);

    if let Some(t) = target {
        // Find if it's a file or function
        let is_file = t.contains('.') && !t.contains("::");

        let graph = if is_file {
            CoChangeGraph::from_metrics(&file_metrics, EntityType::File)
        } else {
            CoChangeGraph::from_metrics(&func_metrics, EntityType::Function)
        };

        let analyzer = ImpactAnalyzer::new(&storage, &graph);
        let impact = analyzer.analyze(t, 10);

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
        if format == Format::Json {
            let env = ResponseEnvelope::success(serde_json::json!({
                "status": "success",
                "stats": stats,
                "top_file_couplings": file_metrics.iter().take(10).collect::<Vec<_>>(),
                "top_function_couplings": func_metrics.iter().take(10).collect::<Vec<_>>(),
                "duration_ms": start_time.elapsed().as_millis(),
            }));
            println!("{}", serde_json::to_string_pretty(&env)?);
        } else {
            format_impact_global(&stats, &file_metrics, &func_metrics);
        }
    }

    Ok(())
}
