use crate::sink::{IndexSink, SinkTouch};
use crate::storage::{AtomicStorageManager, BackgroundWriter, CommitRecord};
use crate::tantivy_index::incremental::IncrementalTantivyWriter;
use crate::vector::FastEmbedGenerator;
use fastembed::EmbeddingModel;

use gitctx_core::config::IndexConfig;
use gitctx_core::error::{CoreError, Result};
use gitctx_core::git::{GitRepo, GitWalker};
use gitctx_core::lineage::LineageResolver;
use gitctx_core::model::CommitId;
use gitctx_core::parser::AstExtractor;
use gitctx_core::sanitize::sanitize_chunk_text;
use gitctx_core::template::{D4TemplateRenderer, FunctionTemplateContext};

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

/// Orchestrates the entire indexing pipeline:
/// Git Walker -> Ast Extractor -> Lineage Resolver -> Storage / Tantivy / Vector Engines
pub struct IndexPipeline;

impl IndexPipeline {
    /// Run the full indexing pipeline for a repository.
    pub fn run(repo_path: &Path, config: &IndexConfig) -> Result<()> {
        let repo = GitRepo::open(repo_path)?;
        let walker = GitWalker::new(&repo)?;

        let extractor = AstExtractor::new();
        let mut resolver = LineageResolver::new();

        // Setup output storage
        let mut storage_mgr = AtomicStorageManager::new(repo_path.join(".gitctx"));
        std::fs::create_dir_all(storage_mgr.base_dir()).map_err(|e| {
            CoreError::InvariantViolation(format!("Failed to create .gitctx dir: {e}"))
        })?;
        std::fs::create_dir_all(storage_mgr.tmp_tantivy_dir()).map_err(|e| {
            CoreError::InvariantViolation(format!("Failed to create tmp tantivy dir: {e}"))
        })?;

        let conn = crate::storage::open_connection(storage_mgr.tmp_db_path())
            .map_err(|e| CoreError::InvariantViolation(format!("Failed to open temp db: {e}")))?;

        let sqlite_writer = BackgroundWriter::new(conn, 100);

        let tantivy_writer =
            IncrementalTantivyWriter::open_with_budget(storage_mgr.tmp_tantivy_dir(), 32_000_000)
                .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // FastEmbed (defaults to BGESmallENV15)
        let embedder = FastEmbedGenerator::new(Some(EmbeddingModel::BGESmallENV15))
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        let mut sink = IndexSink::new(sqlite_writer, tantivy_writer, embedder, 128);
        sink.set_state("schema_version", &config.schema_version.to_string())?;
        sink.set_state("config_hash", config.compute_config_hash().as_str())?;

        let mut touch_counter: i64 = 0;

        // 1. Collect commits and reverse them so history is traversed chronologically (root -> HEAD)
        let commits: Vec<_> = walker.collect::<Result<Vec<_>>>()?;
        let commits: Vec<_> = commits.into_iter().rev().collect();

        // 2. Walk commits in chronological order
        for (idx, commit_info) in commits.into_iter().enumerate() {
            let topo_order = (idx + 1) as u32;
            let commit_id = commit_info.commit_id.clone();

            let commit_record = CommitRecord {
                hash: commit_info.commit_id.to_string(),
                author: commit_info.author_name.clone(),
                email: commit_info.author_email.clone(),
                authored_at: commit_info.timestamp,
                topo_order,
                branch: None,
                parent_hashes: commit_info
                    .parent_ids
                    .iter()
                    .map(CommitId::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
                message: commit_info.message.clone(),
                insertions: 0,
                deletions: 0,
            };

            sink.push_commit(commit_record)?;

            // 3. Resolve Lineages for this commit
            let parent_oid = commit_info
                .parent_ids
                .first()
                .and_then(|id| git2::Oid::from_str(id.as_str()).ok());

            let commit_oid = git2::Oid::from_str(commit_id.as_str())
                .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

            let resolved_commit =
                resolver.process_commit(&repo, commit_oid, parent_oid, &extractor)?;

            // 4. Process touches
            for touch in resolved_commit.touches {
                touch_counter += 1;
                let touch_id = touch_counter as u64;

                let language = extractor
                    .registry()
                    .detect(Path::new(&touch.path), None)
                    .map_or_else(
                        || "rust".to_string(),
                        |b| b.language_id().as_str().to_string(),
                    );

                // Format pseudo-document
                let template_ctx = FunctionTemplateContext {
                    qualname: &touch.qualname,
                    path: &touch.path,
                    author: &commit_info.author_name,
                    date: "Unknown",
                    raw_message: &commit_info.message,
                    added_identifiers: &[],
                    removed_identifiers: &[],
                    signature: Some(touch.signature.as_str()),
                    docstring: touch.docstring.as_deref(),
                };

                let raw_chunk = D4TemplateRenderer::render_function(&template_ctx);
                let sanitized_chunk = sanitize_chunk_text(&raw_chunk);

                sink.push_touch(SinkTouch {
                    touch_id,
                    lineage_id: touch.lineage_id.as_u64(),
                    commit_hash: commit_info.commit_id.to_string(),
                    path: touch.path.clone(),
                    qualified_name: touch.qualname.clone(),
                    language,
                    signature: touch.signature.clone(),
                    chunk_text: sanitized_chunk.sanitized_text,
                    parse_status: touch.parse_status,
                    start_line: touch.start_line,
                    end_line: touch.end_line,
                    commit_message: commit_info.message.clone(),
                })?;
            }
        }

        // 5. Finalize sink, flushing SQLite, Tantivy, and writing dense vectors
        let final_conn = sink.finish(storage_mgr.tmp_vectors_path())?;

        // 6. Precompute co-change metrics and persist into cochange tables
        compute_and_persist_cochange(&final_conn)?;

        // 7. Checkpoint WAL on SQLite connection and close connection
        AtomicStorageManager::checkpoint_and_close(final_conn)
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // 8. Commit atomic swap
        storage_mgr
            .commit_swap()
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        Ok(())
    }
}

struct CommitData {
    email: String,
    authored_at: i64,
    paths: BTreeSet<String>,
    functions: BTreeSet<String>,
}

#[derive(Default)]
struct PairAcc {
    raw_count: u32,
    weighted_sum: f64,
}

/// Compute co-change metrics from indexed commits and persist into SQLite precomputed tables.
fn compute_and_persist_cochange(conn: &rusqlite::Connection) -> Result<()> {
    let mut stmt = conn
        .prepare_cached(
            "SELECT ft.commit_hash, c.email, c.authored_at, ft.path, ft.qualified_name
            FROM function_touches ft
            JOIN commits c ON ft.commit_hash = c.hash
            ORDER BY c.topo_order ASC;",
        )
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

    let mut commits: HashMap<String, CommitData> = HashMap::new();

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

    for r in rows {
        let (hash, email, authored_at, path, qualname) =
            r.map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
        let entry = commits.entry(hash).or_insert_with(|| CommitData {
            email,
            authored_at,
            paths: BTreeSet::new(),
            functions: BTreeSet::new(),
        });
        entry.paths.insert(path);
        entry.functions.insert(qualname);
    }

    let bot_patterns = [
        "[bot]",
        "noreply",
        "dependabot",
        "renovate",
        "github-actions",
    ];
    let mut qualifying = Vec::new();
    let mut max_authored_at = 0i64;

    for (hash, data) in commits {
        if data.paths.len() > 25 {
            continue;
        }
        let email_lower = data.email.to_lowercase();
        if bot_patterns.iter().any(|pat| email_lower.contains(pat)) {
            continue;
        }
        if data.authored_at > max_authored_at {
            max_authored_at = data.authored_at;
        }
        qualifying.push((hash, data));
    }

    let n = qualifying.len();
    if n == 0 {
        return Ok(());
    }

    let half_life_secs = 180.0 * 86_400.0;
    let decay_weight = |authored_at: i64| -> f64 {
        let age_secs = (max_authored_at - authored_at) as f64;
        if age_secs <= 0.0 {
            1.0
        } else {
            let exponent = -(age_secs / half_life_secs);
            (exponent * std::f64::consts::LN_2).exp().clamp(0.0, 1.0)
        }
    };

    let mut file_pairs: HashMap<(String, String), PairAcc> = HashMap::new();
    let mut file_weighted_counts: HashMap<String, f64> = HashMap::new();

    let mut func_pairs: HashMap<(String, String), PairAcc> = HashMap::new();
    let mut func_weighted_counts: HashMap<String, f64> = HashMap::new();

    for (_hash, data) in &qualifying {
        let weight = decay_weight(data.authored_at);

        let paths_vec: Vec<&String> = data.paths.iter().collect();
        for p in &paths_vec {
            *file_weighted_counts.entry((*p).clone()).or_default() += weight;
        }
        for i in 0..paths_vec.len() {
            for j in (i + 1)..paths_vec.len() {
                let a = paths_vec[i];
                let b = paths_vec[j];
                let key = if a <= b {
                    (a.clone(), b.clone())
                } else {
                    (b.clone(), a.clone())
                };
                let acc = file_pairs.entry(key).or_default();
                acc.raw_count += 1;
                acc.weighted_sum += weight;
            }
        }

        let funcs_vec: Vec<&String> = data.functions.iter().collect();
        for f in &funcs_vec {
            *func_weighted_counts.entry((*f).clone()).or_default() += weight;
        }
        for i in 0..funcs_vec.len() {
            for j in (i + 1)..funcs_vec.len() {
                let a = funcs_vec[i];
                let b = funcs_vec[j];
                let key = if a <= b {
                    (a.clone(), b.clone())
                } else {
                    (b.clone(), a.clone())
                };
                let acc = func_pairs.entry(key).or_default();
                acc.raw_count += 1;
                acc.weighted_sum += weight;
            }
        }
    }

    conn.execute_batch("BEGIN TRANSACTION;")
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

    {
        let mut stmt = conn
            .prepare_cached(
                "INSERT OR REPLACE INTO cochange_file_metrics (entity_a, entity_b, support, weighted_support, confidence_a_to_b, confidence_b_to_a, lift) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);"
            )
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        for ((a, b), acc) in file_pairs {
            let min_support = if n < 2 { 1 } else { 2 };
            if acc.raw_count < min_support {
                continue;
            }
            let w_a = *file_weighted_counts.get(&a).unwrap_or(&1.0);
            let w_b = *file_weighted_counts.get(&b).unwrap_or(&1.0);
            let support = acc.raw_count;
            let weighted_support = acc.weighted_sum;
            let conf_a_b = if w_a > 0.0 {
                weighted_support / w_a
            } else {
                0.0
            };
            let conf_b_a = if w_b > 0.0 {
                weighted_support / w_b
            } else {
                0.0
            };
            let lift = if w_a > 0.0 && w_b > 0.0 {
                (weighted_support * n as f64) / (w_a * w_b)
            } else {
                0.0
            };

            stmt.execute(rusqlite::params![
                a,
                b,
                support,
                weighted_support,
                conf_a_b,
                conf_b_a,
                lift
            ])
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
        }
    }

    {
        let mut stmt = conn
            .prepare_cached(
                "INSERT OR REPLACE INTO cochange_function_metrics (entity_a, entity_b, support, weighted_support, confidence_a_to_b, confidence_b_to_a, lift) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);"
            )
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        for ((a, b), acc) in func_pairs {
            let min_support = if n < 2 { 1 } else { 2 };
            if acc.raw_count < min_support {
                continue;
            }
            let w_a = *func_weighted_counts.get(&a).unwrap_or(&1.0);
            let w_b = *func_weighted_counts.get(&b).unwrap_or(&1.0);
            let support = acc.raw_count;
            let weighted_support = acc.weighted_sum;
            let conf_a_b = if w_a > 0.0 {
                weighted_support / w_a
            } else {
                0.0
            };
            let conf_b_a = if w_b > 0.0 {
                weighted_support / w_b
            } else {
                0.0
            };
            let lift = if w_a > 0.0 && w_b > 0.0 {
                (weighted_support * n as f64) / (w_a * w_b)
            } else {
                0.0
            };

            stmt.execute(rusqlite::params![
                a,
                b,
                support,
                weighted_support,
                conf_a_b,
                conf_b_a,
                lift
            ])
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
        }
    }

    conn.execute_batch("COMMIT;")
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

    Ok(())
}
