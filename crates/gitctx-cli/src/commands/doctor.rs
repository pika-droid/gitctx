use crate::envelope::ResponseEnvelope;
use crate::format::format_doctor;
use crate::Format;
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: String,
    pub status: String,
    pub details: String,
}

pub fn run(format: Format) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let mut checks = Vec::new();

    // 1. Git Repository Discovery Check
    let repo_check = match git2::Repository::discover(&current_dir) {
        Ok(repo) => {
            let path_str = repo.path().display().to_string();
            DoctorCheck {
                name: "Git Repository Discovery".to_string(),
                status: "PASS".to_string(),
                details: format!("Repository at {path_str}"),
            }
        }
        Err(e) => DoctorCheck {
            name: "Git Repository Discovery".to_string(),
            status: "FAIL".to_string(),
            details: format!("No git repository found: {e}"),
        },
    };
    let is_git = repo_check.status == "PASS";
    checks.push(repo_check);

    // 2. Index Directory & SQLite Database
    let dot_gitctx = current_dir.join(".gitctx");
    let db_path = dot_gitctx.join("index.db");

    if db_path.exists() {
        let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "BGESmallENV15", "code", 500, 1);
        let storage_res = StorageEngine::open(&db_path, &config);

        match storage_res {
            Ok(storage) => {
                let conn = storage.connection();

                // SQLite Integrity PRAGMA
                let integrity: Result<String, _> =
                    conn.query_row("PRAGMA integrity_check;", [], |r| r.get(0));
                match integrity {
                    Ok(ref s) if s == "ok" => {
                        checks.push(DoctorCheck {
                            name: "SQLite Index Integrity".to_string(),
                            status: "PASS".to_string(),
                            details: "Database integrity PRAGMA check passed".to_string(),
                        });
                    }
                    Ok(ref s) => {
                        checks.push(DoctorCheck {
                            name: "SQLite Index Integrity".to_string(),
                            status: "FAIL".to_string(),
                            details: format!("Integrity check returned: {s}"),
                        });
                    }
                    Err(e) => {
                        checks.push(DoctorCheck {
                            name: "SQLite Index Integrity".to_string(),
                            status: "FAIL".to_string(),
                            details: format!("Integrity check error: {e}"),
                        });
                    }
                }

                // Schema Version Check
                let db_schema: Result<u32, _> = conn.query_row(
                    "SELECT value FROM index_state WHERE key = 'schema_version'",
                    [],
                    |r| {
                        let val: String = r.get(0)?;
                        val.parse::<u32>().map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                0,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })
                    },
                );
                match db_schema {
                    Ok(v) if v == CURRENT_SCHEMA_VERSION => {
                        checks.push(DoctorCheck {
                            name: "Schema Version Compatibility".to_string(),
                            status: "PASS".to_string(),
                            details: format!("v{v} (matches current engine schema)"),
                        });
                    }
                    Ok(v) => {
                        checks.push(DoctorCheck {
                            name: "Schema Version Compatibility".to_string(),
                            status: "FAIL".to_string(),
                            details: format!("v{v} (engine expects v{CURRENT_SCHEMA_VERSION})"),
                        });
                    }
                    Err(e) => {
                        checks.push(DoctorCheck {
                            name: "Schema Version Compatibility".to_string(),
                            status: "FAIL".to_string(),
                            details: format!("Could not read schema_version: {e}"),
                        });
                    }
                }

                // Config Hash Match
                let db_hash: Result<String, _> = conn.query_row(
                    "SELECT value FROM index_state WHERE key = 'config_hash'",
                    [],
                    |r| r.get(0),
                );
                let current_hash = config.compute_config_hash();
                match db_hash {
                    Ok(ref h) if h == current_hash.as_str() => {
                        checks.push(DoctorCheck {
                            name: "Config Hash Match".to_string(),
                            status: "PASS".to_string(),
                            details: format!("Hash matches: {h}"),
                        });
                    }
                    Ok(ref h) => {
                        checks.push(DoctorCheck {
                            name: "Config Hash Match".to_string(),
                            status: "WARN".to_string(),
                            details: format!(
                                "Index hash ({h}) != Current config hash ({})",
                                current_hash.as_str()
                            ),
                        });
                    }
                    Err(e) => {
                        checks.push(DoctorCheck {
                            name: "Config Hash Match".to_string(),
                            status: "FAIL".to_string(),
                            details: format!("Could not read config_hash: {e}"),
                        });
                    }
                }

                // Vector Storage Consistency
                let vectors_path = dot_gitctx.join("vectors.bin");
                if vectors_path.exists() {
                    match std::fs::metadata(&vectors_path) {
                        Ok(meta) => {
                            checks.push(DoctorCheck {
                                name: "Vector Storage Consistency".to_string(),
                                status: "PASS".to_string(),
                                details: format!("vectors.bin exists ({} bytes)", meta.len()),
                            });
                        }
                        Err(e) => {
                            checks.push(DoctorCheck {
                                name: "Vector Storage Consistency".to_string(),
                                status: "FAIL".to_string(),
                                details: format!("Failed to read vectors.bin metadata: {e}"),
                            });
                        }
                    }
                } else {
                    checks.push(DoctorCheck {
                        name: "Vector Storage Consistency".to_string(),
                        status: "WARN".to_string(),
                        details: "vectors.bin not found (dense search unavailable)".to_string(),
                    });
                }

                // Parse Status Distribution
                let stmt_res = conn.prepare(
                    "SELECT parse_status, count(*) FROM function_touches GROUP BY parse_status",
                );
                if let Ok(mut stmt) = stmt_res {
                    let rows_res = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)));
                    if let Ok(rows) = rows_res {
                        let mut counts: Vec<(String, i64)> = Vec::new();
                        for pair in rows.flatten() {
                            counts.push(pair);
                        }
                        let total: i64 = counts.iter().map(|(_, c)| c).sum();
                        let summary = counts
                            .iter()
                            .map(|(status, c)| format!("{status}: {c}"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        let has_failures = counts.iter().any(|(s, _)| s == "failed");
                        checks.push(DoctorCheck {
                            name: "Parse Status Distribution".to_string(),
                            status: if has_failures {
                                "WARN".to_string()
                            } else {
                                "PASS".to_string()
                            },
                            details: format!("Total touches: {total} ({summary})"),
                        });
                    }
                }

                // Index Freshness vs HEAD
                if is_git {
                    if let Ok(repo) = git2::Repository::discover(&current_dir) {
                        let head_commit = repo
                            .head()
                            .ok()
                            .and_then(|h| h.target())
                            .map(|t| t.to_string());
                        let latest_indexed: Result<Option<String>, _> = conn
                            .query_row(
                                "SELECT hash FROM commits ORDER BY authored_at DESC LIMIT 1",
                                [],
                                |r| r.get(0),
                            )
                            .optional();

                        match (head_commit, latest_indexed) {
                            (Some(head), Ok(Some(indexed))) => {
                                if head == indexed
                                    || indexed.starts_with(&head)
                                    || head.starts_with(&indexed)
                                {
                                    checks.push(DoctorCheck {
                                        name: "Index Freshness".to_string(),
                                        status: "PASS".to_string(),
                                        details: format!(
                                            "Index is aligned with HEAD ({})",
                                            &head[..7.min(head.len())]
                                        ),
                                    });
                                } else {
                                    checks.push(DoctorCheck {
                                        name: "Index Freshness".to_string(),
                                        status: "WARN".to_string(),
                                        details: format!(
                                            "HEAD ({}) != Indexed HEAD ({})",
                                            &head[..7.min(head.len())],
                                            &indexed[..7.min(indexed.len())]
                                        ),
                                    });
                                }
                            }
                            (Some(head), _) => {
                                checks.push(DoctorCheck {
                                    name: "Index Freshness".to_string(),
                                    status: "WARN".to_string(),
                                    details: format!(
                                        "No commits in index, HEAD is at {}",
                                        &head[..7.min(head.len())]
                                    ),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(e) => {
                checks.push(DoctorCheck {
                    name: "SQLite Storage Engine".to_string(),
                    status: "FAIL".to_string(),
                    details: format!("Failed to open storage engine: {e}"),
                });
            }
        }
    } else {
        checks.push(DoctorCheck {
            name: "SQLite Index Database".to_string(),
            status: "FAIL".to_string(),
            details: format!(
                "Database not found at {}. Run 'gitctx index'",
                db_path.display()
            ),
        });
        checks.push(DoctorCheck {
            name: "Schema Version Compatibility".to_string(),
            status: "FAIL".to_string(),
            details: "Skipped (no database found)".to_string(),
        });
        checks.push(DoctorCheck {
            name: "Config Hash Match".to_string(),
            status: "FAIL".to_string(),
            details: "Skipped (no database found)".to_string(),
        });
        checks.push(DoctorCheck {
            name: "Vector Storage Consistency".to_string(),
            status: "FAIL".to_string(),
            details: "Skipped (no database found)".to_string(),
        });
        checks.push(DoctorCheck {
            name: "Parse Status Distribution".to_string(),
            status: "FAIL".to_string(),
            details: "Skipped (no database found)".to_string(),
        });
        checks.push(DoctorCheck {
            name: "Index Freshness".to_string(),
            status: "FAIL".to_string(),
            details: "Skipped (no database found)".to_string(),
        });
    }

    if format == Format::Json {
        let all_pass = checks.iter().all(|c| c.status == "PASS");
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": if all_pass { "ok" } else { "degraded" },
            "checks": checks,
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        format_doctor(&checks);
    }

    Ok(())
}
