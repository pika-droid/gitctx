use crate::envelope::ResponseEnvelope;
use crate::format::format_evolution;
use crate::Format;
use anyhow::{Context, Result};
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use gitctx_query::chain::ChainAssembler;
use std::path::Path;
use std::time::Instant;

pub fn run(identifier: &str, format: Format) -> Result<()> {
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

    // Lookup lineage_id from identifier
    let conn = storage.connection();
    let lineage_id = ChainAssembler::resolve_lineage_id(conn, identifier)
        .context("Failed to query lineage id")?;

    let Some(lineage_id) = lineage_id else {
        if format == Format::Json {
            let env = ResponseEnvelope::<()>::error(
                "NOT_FOUND",
                format!("No symbol found matching {identifier}"),
            );
            println!("{}", serde_json::to_string_pretty(&env)?);
        } else {
            println!("No symbol found matching {identifier}");
        }
        return Ok(());
    };

    let chain = ChainAssembler::assemble_lineage_chain(conn, lineage_id)
        .context("Failed to assemble lineage chain")?;

    let duration = start_time.elapsed();

    if format == Format::Json {
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": "success",
            "identifier": identifier,
            "chain": chain,
            "duration_ms": duration.as_millis(),
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        format_evolution(&chain);
    }

    Ok(())
}
