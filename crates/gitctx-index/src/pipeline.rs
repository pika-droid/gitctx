use crate::storage::{
    AtomicStorageManager, BackgroundWriter, CommitRecord, LineageRecord, TouchRecord,
};
use crate::tantivy_index::incremental::IncrementalTantivyWriter;
use crate::tantivy_index::schema::TantivyTouchDoc;
use crate::vector::{write_vector_file_atomic, FastEmbedGenerator};
use fastembed::EmbeddingModel;

use gitctx_core::config::IndexConfig;
use gitctx_core::error::{CoreError, Result};
use gitctx_core::git::{GitRepo, GitWalker};
use gitctx_core::lineage::LineageResolver;
use gitctx_core::model::{CommitId, ParseStatus};
use gitctx_core::parser::AstExtractor;
use gitctx_core::sanitize::sanitize_chunk_text;
use gitctx_core::template::{D4TemplateRenderer, FunctionTemplateContext};

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
        let conn = crate::storage::open_connection(storage_mgr.tmp_db_path())
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        let sqlite_writer = BackgroundWriter::new(conn, 100);
        sqlite_writer
            .set_state("schema_version", config.schema_version.to_string())
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
        sqlite_writer
            .set_state("config_hash", config.compute_config_hash().as_str())
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        let mut tantivy_writer =
            IncrementalTantivyWriter::open_with_budget(storage_mgr.tmp_tantivy_dir(), 32_000_000)
                .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // FastEmbed (may take time to load model)
        // Note: For now we default to BGESmallENV15
        let mut embedder = FastEmbedGenerator::new(Some(EmbeddingModel::BGESmallENV15))
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        let mut vector_data = Vec::new();
        let mut touch_counter: i64 = 0;

        let mut commit_records = Vec::new();

        // 1. Walk commits
        for commit_res in walker {
            let commit_info = commit_res?;
            let commit_id = commit_info.commit_id.clone();

            commit_records.push(CommitRecord {
                hash: commit_info.commit_id.to_string(),
                author: commit_info.author_name.clone(),
                email: commit_info.author_email.clone(),
                authored_at: commit_info.timestamp,
                topo_order: 1, // Need actual topo_order logic if we were strictly topological
                branch: None,  // Simplified
                parent_hashes: commit_info
                    .parent_ids
                    .iter()
                    .map(CommitId::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
                message: commit_info.message.clone(),
                insertions: 0, // Should be extracted from diff if needed
                deletions: 0,
            });

            // 2. Resolve Lineages for this commit
            let parent_oid = commit_info
                .parent_ids
                .first()
                .and_then(|id| git2::Oid::from_str(id.as_str()).ok());

            let commit_oid = git2::Oid::from_str(commit_id.as_str())
                .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

            let resolved_commit =
                resolver.process_commit(&repo, commit_oid, parent_oid, &extractor)?;

            // 3. Process touches
            for touch in resolved_commit.touches {
                touch_counter += 1;
                let touch_id = touch_counter;

                // Format pseudo-document
                let template_ctx = FunctionTemplateContext {
                    qualname: &touch.qualname,
                    path: &touch.path,
                    author: &commit_info.author_name,
                    date: "Unknown", // Simplification
                    raw_message: &commit_info.message,
                    added_identifiers: &[],
                    removed_identifiers: &[],
                    signature: Some(touch.signature.as_str()),
                    docstring: touch.docstring.as_deref(),
                };

                let raw_chunk = D4TemplateRenderer::render_function(&template_ctx);
                let sanitized_chunk = sanitize_chunk_text(&raw_chunk);

                // Write to SQLite
                sqlite_writer
                    .push_touches(vec![TouchRecord {
                        touch_id: Some(touch_id),
                        lineage_id: touch.lineage_id.as_u64(),
                        commit_hash: commit_info.commit_id.to_string(),
                        path: touch.path.clone(),
                        qualified_name: touch.qualname.clone(),
                        arity: 0, // Simplify
                        change_type: "modify".to_string(),
                        signature_before: None,
                        signature_after: Some(touch.signature.clone()),
                        fingerprint: None,
                        chunk_text: sanitized_chunk.sanitized_text.clone(),
                        parse_status: match touch.parse_status {
                            ParseStatus::Ok => "ok".to_string(),
                            ParseStatus::Degraded => "degraded".to_string(),
                            ParseStatus::Failed => "failed".to_string(),
                        },
                        start_line: touch.start_line,
                        end_line: touch.end_line,
                    }])
                    .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

                // Write to Tantivy
                let doc = TantivyTouchDoc {
                    touch_id: touch_id as u64,
                    lineage_id: touch.lineage_id.as_u64(),
                    commit_hash: commit_info.commit_id.to_string(),
                    qualified_name: touch.qualname.clone(),
                    path: touch.path.clone(),
                    commit_message: commit_info.message.clone(),
                    chunk_text: sanitized_chunk.sanitized_text.clone(),
                };
                tantivy_writer
                    .add_touch(&doc)
                    .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

                // Generate Embedding
                let embeddings = embedder
                    .embed_texts(&[sanitized_chunk.sanitized_text.clone()], None)
                    .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

                if let Some(vec_f32) = embeddings.into_iter().next() {
                    vector_data.extend_from_slice(&vec_f32);
                }
            }
        }

        sqlite_writer
            .push_commits(commit_records)
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // Also push lineages
        let mut lineages = Vec::new();
        for (path, syms) in resolver.active_symbols() {
            for sym in syms {
                if let Some(lid) = sym.lineage_id {
                    lineages.push(LineageRecord {
                        lineage_id: lid.as_u64(),
                        current_path: path.clone(),
                        current_qualified_name: sym.qualname.clone(),
                        language: "rust".to_string(), // Simplify
                    });
                }
            }
        }
        sqlite_writer
            .push_lineages(lineages)
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // 4. Finalize writers
        let final_conn = sqlite_writer
            .finish()
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
        AtomicStorageManager::checkpoint_and_close(final_conn)
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        tantivy_writer
            .finish()
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // 5. Write vectors
        let dimension = embedder.dimension() as u32;
        write_vector_file_atomic(
            storage_mgr.tmp_vectors_path(),
            &vector_data,
            dimension,
            touch_counter as u32,
        )
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // 6. Commit swap
        storage_mgr
            .commit_swap()
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        Ok(())
    }
}
