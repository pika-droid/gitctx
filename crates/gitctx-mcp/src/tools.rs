//! MCP tool implementations per MCP-01.
//!
//! Exposes five tools: `search_history`, `function_evolution`, `change_impact`,
//! `who_knows_this`, and `commit_detail`. Each tool reuses the query-layer types
//! from `gitctx-query` and applies budget formatting from `budget.rs`.

use crate::budget::{self, BudgetMode};
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use gitctx_index::tantivy_index::TantivyEngine;
use gitctx_index::vector::{FastEmbedGenerator, MmapVectorStorage};
use gitctx_query::chain::ChainAssembler;
use gitctx_query::cochange::{
    CoChangeConfig, CoChangeExtractor, CoChangeGraph, EntityType, ImpactAnalyzer, MetricsComputer,
};
use gitctx_query::retrieval::HybridRetriever;
use rusqlite::OptionalExtension;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// MCP tool descriptor for `tools/list` response.
pub fn tool_definitions() -> serde_json::Value {
    serde_json::json!({
        "tools": [
            {
                "name": "search_history",
                "description": "Search the indexed Git history for functions, changes, and commits matching a natural language or identifier query. Returns ranked results with citations.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Natural language or identifier query to search the codebase history"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["compact", "full"],
                            "description": "Token budget mode. 'compact' (default) returns citations only. 'full' includes chunk text."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 10)",
                            "default": 10
                        }
                    },
                    "required": ["query"]
                }
            },
            {
                "name": "function_evolution",
                "description": "Track the complete evolutionary history of a function across renames, moves, and refactors. Returns a chronological timeline of all changes.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "identifier": {
                            "type": "string",
                            "description": "Function name or qualified name (e.g. 'parse_config' or 'Config::parse')"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["compact", "full"],
                            "description": "Token budget mode. 'compact' (default) returns citations only. 'full' includes chunk text."
                        }
                    },
                    "required": ["identifier"]
                }
            },
            {
                "name": "change_impact",
                "description": "Analyze the risk and co-change coupling impact of a file or function. Shows statistically coupled entities that frequently change together.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "target": {
                            "type": "string",
                            "description": "File path or function name to analyze impact for"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["compact", "full"],
                            "description": "Token budget mode"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of coupled entities to return (default: 10)",
                            "default": 10
                        }
                    },
                    "required": ["target"]
                }
            },
            {
                "name": "who_knows_this",
                "description": "Find the top contributors who have most frequently modified a function or file. Useful for identifying domain experts and code owners.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "identifier": {
                            "type": "string",
                            "description": "Function name, qualified name, or file path"
                        }
                    },
                    "required": ["identifier"]
                }
            },
            {
                "name": "commit_detail",
                "description": "Show detailed information about a specific commit including all functions it touched, the author, message, and parent commits.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "hash": {
                            "type": "string",
                            "description": "Full or abbreviated commit hash"
                        },
                        "mode": {
                            "type": "string",
                            "enum": ["compact", "full"],
                            "description": "Token budget mode"
                        }
                    },
                    "required": ["hash"]
                }
            }
        ]
    })
}

/// Shared engine context holding all storage backends.
pub struct ToolContext {
    storage: StorageEngine,
    tantivy: Option<TantivyEngine>,
    vectors: Option<MmapVectorStorage>,
    repo_path: std::path::PathBuf,
}

impl ToolContext {
    /// Initialize from a repository working directory.
    pub fn open(repo_path: &Path) -> Result<Self, String> {
        let db_path = repo_path.join(".gitctx").join("index.db");
        let tantivy_path = repo_path.join(".gitctx").join("tantivy");
        let vectors_path = repo_path.join(".gitctx").join("vectors.bin");

        if !db_path.exists() {
            return Err(format!(
                "Index database not found at {}. Run 'gitctx index' first.",
                db_path.display()
            ));
        }

        let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "BGESmallENV15", "code", 500, 1);
        let storage = StorageEngine::open(&db_path, &config)
            .map_err(|e| format!("Failed to open storage: {e}"))?;
        let tantivy = TantivyEngine::open(&tantivy_path).ok();
        let vectors = MmapVectorStorage::open(&vectors_path).ok();

        Ok(Self {
            storage,
            tantivy,
            vectors,
            repo_path: repo_path.to_path_buf(),
        })
    }

    /// Check if the index is stale (HEAD has advanced past indexed commit) per MCP-04.
    pub fn check_stale_index(&self) -> Option<serde_json::Value> {
        let repo = git2::Repository::discover(&self.repo_path).ok()?;
        let head = repo.head().ok()?;
        let head_oid = head.target()?;
        let head_hex = head_oid.to_string();

        let conn = self.storage.connection();
        let indexed_head: Option<String> = conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'head_at_index';",
                [],
                |row| row.get(0),
            )
            .optional()
            .ok()?;

        if let Some(ref indexed) = indexed_head {
            if *indexed != head_hex {
                // Count commits ahead
                let mut revwalk = repo.revwalk().ok()?;
                revwalk.push(head_oid).ok()?;
                let indexed_oid = git2::Oid::from_str(indexed).ok()?;
                revwalk.hide(indexed_oid).ok()?;
                let commits_behind = revwalk.count();

                return Some(serde_json::json!({
                    "stale_index": true,
                    "commits_behind": commits_behind,
                    "indexed_head": indexed,
                    "current_head": head_hex,
                }));
            }
        }
        None
    }

    // ─── Tool: search_history ───────────────────────────────────────────

    pub fn search_history(&self, params: &serde_json::Value) -> Result<serde_json::Value, String> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: query")?;
        let mode = BudgetMode::from_str_opt(params.get("mode").and_then(|v| v.as_str()));
        let limit = params
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10) as usize;

        // Embed query for dense vector search
        let query_vector = if self.vectors.is_some() {
            let mut embedder = FastEmbedGenerator::new(None)
                .map_err(|e| format!("Failed to initialize embedder: {e}"))?;
            let mut embeddings = embedder
                .embed_texts(&[query.to_string()], None)
                .map_err(|e| format!("Failed to embed query: {e}"))?;
            embeddings.pop()
        } else {
            None
        };

        let retriever = HybridRetriever::new(
            self.storage.connection(),
            self.tantivy.as_ref(),
            self.vectors.as_ref(),
        );

        let results = retriever
            .search(query, query_vector.as_deref(), limit)
            .map_err(|e| format!("Search failed: {e}"))?;

        let formatted: Vec<serde_json::Value> = results
            .iter()
            .map(|f| budget::format_fact(f, mode))
            .collect();

        let mut response = serde_json::json!({
            "query": query,
            "result_count": formatted.len(),
            "results": formatted,
        });

        if let Some(stale) = self.check_stale_index() {
            response
                .as_object_mut()
                .unwrap()
                .insert("index_status".to_string(), stale);
        }

        Ok(response)
    }

    // ─── Tool: function_evolution ────────────────────────────────────────

    pub fn function_evolution(
        &self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let identifier = params
            .get("identifier")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: identifier")?;
        let mode = BudgetMode::from_str_opt(params.get("mode").and_then(|v| v.as_str()));

        let conn = self.storage.connection();
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
            .map_err(|e| format!("Lineage lookup failed: {e}"))?;

        let Some(lineage_id) = lineage_id else {
            return Ok(serde_json::json!({
                "error": "not_found",
                "message": format!("No symbol found matching '{identifier}'"),
            }));
        };

        let chain = ChainAssembler::assemble_lineage_chain(conn, lineage_id)
            .map_err(|e| format!("Chain assembly failed: {e}"))?;

        let mut response = budget::format_chain(&chain, mode);

        if let Some(stale) = self.check_stale_index() {
            response
                .as_object_mut()
                .unwrap()
                .insert("index_status".to_string(), stale);
        }

        Ok(response)
    }

    // ─── Tool: change_impact ────────────────────────────────────────────

    pub fn change_impact(&self, params: &serde_json::Value) -> Result<serde_json::Value, String> {
        let target = params
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: target")?;
        let _mode = BudgetMode::from_str_opt(params.get("mode").and_then(|v| v.as_str()));
        let limit = params
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10) as usize;

        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let now_epoch = i64::try_from(secs).unwrap_or(i64::MAX);
        let cochange_config = CoChangeConfig::with_reference_epoch(now_epoch);

        let extractor = CoChangeExtractor::new(&self.storage, &cochange_config);
        let (file_pairs, func_pairs, stats) = extractor
            .extract()
            .map_err(|e| format!("Co-change extraction failed: {e}"))?;

        // Heuristic: targets with dots and no :: are files
        let is_file = target.contains('.') && !target.contains("::");

        let (metrics, entity_type) = if is_file {
            let m =
                MetricsComputer::compute(&file_pairs, stats.qualifying_commits, &cochange_config);
            (m, EntityType::File)
        } else {
            let m =
                MetricsComputer::compute(&func_pairs, stats.qualifying_commits, &cochange_config);
            (m, EntityType::Function)
        };

        let graph = CoChangeGraph::from_metrics(&metrics, entity_type);
        let analyzer = ImpactAnalyzer::new(&self.storage, &graph);
        let impact = analyzer.analyze(target, limit);

        let mut response = serde_json::json!({
            "target": target,
            "risk_score": impact.risk_score,
            "followup_fix_count": impact.followup_fix_count,
            "coupled_entities": impact.coupled_entities.iter().map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "confidence": c.confidence,
                    "lift": c.lift,
                    "support": c.support,
                })
            }).collect::<Vec<_>>(),
        });

        if let Some(stale) = self.check_stale_index() {
            response
                .as_object_mut()
                .unwrap()
                .insert("index_status".to_string(), stale);
        }

        Ok(response)
    }

    // ─── Tool: who_knows_this ───────────────────────────────────────────

    pub fn who_knows_this(&self, params: &serde_json::Value) -> Result<serde_json::Value, String> {
        let identifier = params
            .get("identifier")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: identifier")?;

        let conn = self.storage.connection();

        // Check if this is a file path or a function name
        let is_file =
            identifier.contains('/') || identifier.contains('\\') || identifier.contains('.');

        let authors: Vec<(String, i64)> = if is_file {
            // Query authors by file path
            let mut stmt = conn
                .prepare_cached(
                    "SELECT c.author, COUNT(*) as touch_count
                     FROM function_touches ft
                     JOIN commits c ON ft.commit_hash = c.hash
                     WHERE ft.path = ?1 OR ft.path LIKE ?2
                     GROUP BY c.author
                     ORDER BY touch_count DESC
                     LIMIT 10;",
                )
                .map_err(|e| format!("Query preparation failed: {e}"))?;

            let like_pat = format!("%{identifier}");
            let rows = stmt
                .query_map(rusqlite::params![identifier, like_pat], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .map_err(|e| format!("Query failed: {e}"))?;

            let mut results = Vec::new();
            for r in rows {
                results.push(r.map_err(|e| format!("Row error: {e}"))?);
            }
            results
        } else {
            // Query authors by function qualified_name
            let like_suffix = format!("%::{identifier}");
            let like_dot = format!("%.{identifier}");

            let mut stmt = conn
                .prepare_cached(
                    "SELECT c.author, COUNT(*) as touch_count
                     FROM function_touches ft
                     JOIN commits c ON ft.commit_hash = c.hash
                     WHERE ft.qualified_name = ?1 OR ft.qualified_name LIKE ?2 OR ft.qualified_name LIKE ?3
                     GROUP BY c.author
                     ORDER BY touch_count DESC
                     LIMIT 10;",
                )
                .map_err(|e| format!("Query preparation failed: {e}"))?;

            let rows = stmt
                .query_map(
                    rusqlite::params![identifier, like_suffix, like_dot],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(|e| format!("Query failed: {e}"))?;

            let mut results = Vec::new();
            for r in rows {
                results.push(r.map_err(|e| format!("Row error: {e}"))?);
            }
            results
        };

        if authors.is_empty() {
            return Ok(serde_json::json!({
                "error": "not_found",
                "message": format!("No history found for '{identifier}'"),
            }));
        }

        let total_touches: i64 = authors.iter().map(|(_, c)| c).sum();

        Ok(serde_json::json!({
            "identifier": identifier,
            "total_touches": total_touches,
            "experts": authors.iter().map(|(author, count)| {
                serde_json::json!({
                    "author": author,
                    "touch_count": count,
                    "percentage": (*count as f64 / total_touches as f64 * 100.0).round(),
                })
            }).collect::<Vec<_>>(),
        }))
    }

    // ─── Tool: commit_detail ────────────────────────────────────────────

    pub fn commit_detail(&self, params: &serde_json::Value) -> Result<serde_json::Value, String> {
        let hash = params
            .get("hash")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: hash")?;
        let mode = BudgetMode::from_str_opt(params.get("mode").and_then(|v| v.as_str()));

        let conn = self.storage.connection();
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
            .map_err(|e| format!("Commit query failed: {e}"))?;

        let Some((full_hash, author, email, authored_at, message, parents)) = commit_info else {
            return Ok(serde_json::json!({
                "error": "not_found",
                "message": format!("No commit found matching '{hash}'"),
            }));
        };

        // Fetch touched functions
        let mut stmt = conn
            .prepare_cached(
                "SELECT path, qualified_name, change_type, start_line, end_line, chunk_text
                 FROM function_touches
                 WHERE commit_hash = ?1
                 ORDER BY path ASC, qualified_name ASC;",
            )
            .map_err(|e| format!("Touches query failed: {e}"))?;

        let rows = stmt
            .query_map(rusqlite::params![full_hash], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(|e| format!("Touches query failed: {e}"))?;

        let mut touches = Vec::new();
        for r in rows {
            let (path, qualname, change, start, end, chunk) =
                r.map_err(|e| format!("Row error: {e}"))?;
            let touch_val = match mode {
                BudgetMode::Compact => serde_json::json!({
                    "path": path,
                    "qualified_name": qualname,
                    "change_type": change,
                    "start_line": start,
                    "end_line": end,
                }),
                BudgetMode::Full => serde_json::json!({
                    "path": path,
                    "qualified_name": qualname,
                    "change_type": change,
                    "start_line": start,
                    "end_line": end,
                    "chunk_text": chunk,
                }),
            };
            touches.push(touch_val);
        }

        let parent_list: Vec<&str> = parents.split(',').filter(|s| !s.is_empty()).collect();

        let mut response = serde_json::json!({
            "commit": {
                "hash": full_hash,
                "author": author,
                "email": email,
                "authored_at": authored_at,
                "message": if mode == BudgetMode::Compact {
                    serde_json::Value::String(message.lines().next().unwrap_or("").to_string())
                } else {
                    serde_json::Value::String(message)
                },
                "parents": parent_list,
            },
            "touches": touches,
            "touch_count": touches.len(),
        });

        if let Some(stale) = self.check_stale_index() {
            response
                .as_object_mut()
                .unwrap()
                .insert("index_status".to_string(), stale);
        }

        Ok(response)
    }
}
