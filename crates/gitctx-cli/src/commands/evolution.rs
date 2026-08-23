use crate::envelope::ResponseEnvelope;
use crate::Format;
use anyhow::{Context, Result};
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use gitctx_query::chain::ChainAssembler;
use rusqlite::OptionalExtension;
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

    if format == Format::Table {
        println!("Tracking evolution of: {identifier}");
    }

    // Lookup lineage_id from identifier
    let conn = storage.connection();
    let like_suffix = format!("%::{identifier}");
    let like_dot = format!("%.{identifier}");

    let lineage_id: Option<u64> = conn
        .query_row(
            "SELECT lineage_id FROM function_touches 
         WHERE qualified_name = ?1 OR qualified_name LIKE ?2 OR qualified_name LIKE ?3
         ORDER BY touch_id DESC LIMIT 1;",
            rusqlite::params![identifier, like_suffix, like_dot],
            |row| row.get(0),
        )
        .optional()
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
        println!(
            "Evolution for {} ({} touches):",
            chain.current_qualified_name,
            chain.nodes.len()
        );
        for node in &chain.nodes {
            println!(
                "\nCommit {} by {} on {}",
                node.touch.commit_hash, node.commit_author, node.commit_authored_at
            );
            println!(
                "Message: {}",
                node.commit_message.lines().next().unwrap_or("")
            );
            println!(
                "Path: {} @ {}:{}",
                node.touch.path, node.touch.start_line, node.touch.end_line
            );
            println!("Change Type: {}", node.touch.change_type);
        }
        println!("\nFound evolution in {duration:?}");
    }

    Ok(())
}
