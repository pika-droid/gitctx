use crate::envelope::ResponseEnvelope;
use crate::format::format_show;
use crate::Format;
use anyhow::{Context, Result};
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use rusqlite::OptionalExtension;
use std::path::Path;

pub fn run(hash: &str, format: Format) -> Result<()> {
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

    let conn = storage.connection();

    // Find commit
    let like_hash = format!("{hash}%");
    let commit_info: Option<(String, String, String, i64, String, String)> = conn
        .query_row(
            "SELECT hash, author, email, authored_at, message, parent_hashes
         FROM commits 
         WHERE hash LIKE ?1
         ORDER BY authored_at DESC LIMIT 1;",
            rusqlite::params![like_hash],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()
        .context("Failed to query commit")?;

    let Some((full_hash, author, email, authored_at, message, parents)) = commit_info else {
        if format == Format::Json {
            let env = ResponseEnvelope::<()>::error(
                "NOT_FOUND",
                format!("No commit found matching {hash}"),
            );
            println!("{}", serde_json::to_string_pretty(&env)?);
        } else {
            println!("No commit found matching {hash}");
        }
        return Ok(());
    };

    // Find touched functions
    let mut stmt = conn.prepare_cached(
        "SELECT path, qualified_name, change_type, start_line, end_line
         FROM function_touches
         WHERE commit_hash = ?1
         ORDER BY path ASC, qualified_name ASC;",
    )?;

    let rows = stmt.query_map(rusqlite::params![full_hash], |row| {
        let path: String = row.get(0)?;
        let qualname: String = row.get(1)?;
        let change: String = row.get(2)?;
        let start: i64 = row.get(3)?;
        let end: i64 = row.get(4)?;
        Ok((path, qualname, change, start, end))
    })?;

    let mut touches = Vec::new();
    for r in rows {
        touches.push(r?);
    }

    if format == Format::Json {
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": "success",
            "commit": {
                "hash": full_hash,
                "author": author,
                "email": email,
                "authored_at": authored_at,
                "message": message,
                "parents": parents.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
            },
            "touches": touches.iter().map(|(p, q, c, s, e)| {
                serde_json::json!({
                    "path": p,
                    "qualified_name": q,
                    "change_type": c,
                    "start_line": s,
                    "end_line": e,
                })
            }).collect::<Vec<_>>()
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        format_show(
            &full_hash,
            &author,
            &email,
            authored_at,
            &message,
            &parents,
            &touches,
        );
    }

    Ok(())
}
