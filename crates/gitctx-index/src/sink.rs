use crate::storage::{
    BackgroundWriter, CommitRecord, EmbeddingRecord, FileRecord, LineageRecord, TouchRecord,
};
use crate::tantivy_index::incremental::IncrementalTantivyWriter;
use crate::tantivy_index::schema::TantivyTouchDoc;
use crate::vector::{write_vector_file_atomic, EmbeddingInput, FastEmbedGenerator};
use gitctx_core::error::{CoreError, Result};
use gitctx_core::model::ParseStatus;
use std::path::Path;

/// Touch item queued for batched indexing into SQLite, Tantivy, and Vector store.
#[derive(Debug, Clone)]
pub struct SinkTouch {
    pub touch_id: u64,
    pub lineage_id: u64,
    pub commit_hash: String,
    pub path: String,
    pub qualified_name: String,
    pub language: String,
    pub signature: String,
    pub chunk_text: String,
    pub parse_status: ParseStatus,
    pub start_line: usize,
    pub end_line: usize,
    pub commit_message: String,
}

/// Unified deep ingestion sink coordinating SQLite, Tantivy, and batched dense vector writes.
pub struct IndexSink {
    sqlite_writer: BackgroundWriter,
    tantivy_writer: IncrementalTantivyWriter,
    embedder: FastEmbedGenerator,
    batch_size: usize,
    buffer: Vec<SinkTouch>,
    vector_data: Vec<f32>,
    row_offset_counter: u32,
    dimension: usize,
}

impl IndexSink {
    /// Create a new `IndexSink` wrapping the storage backends with a specified batch size.
    pub fn new(
        sqlite_writer: BackgroundWriter,
        tantivy_writer: IncrementalTantivyWriter,
        embedder: FastEmbedGenerator,
        batch_size: usize,
    ) -> Self {
        let dimension = embedder.dimension();
        Self {
            sqlite_writer,
            tantivy_writer,
            embedder,
            batch_size: if batch_size == 0 { 128 } else { batch_size },
            buffer: Vec::new(),
            vector_data: Vec::new(),
            row_offset_counter: 0,
            dimension,
        }
    }

    /// Dispatch a commit record directly to SQLite.
    pub fn push_commit(&self, commit: CommitRecord) -> Result<()> {
        self.sqlite_writer
            .push_commits(vec![commit])
            .map_err(CoreError::InvariantViolation)
    }

    /// Set an index state key-value pair in SQLite.
    pub fn set_state(&self, key: &str, value: &str) -> Result<()> {
        self.sqlite_writer
            .set_state(key, value)
            .map_err(CoreError::InvariantViolation)
    }

    /// Enqueue a resolved function touch. Flushes batches when reaching `batch_size`.
    pub fn push_touch(&mut self, touch: SinkTouch) -> Result<()> {
        self.buffer.push(touch);
        if self.buffer.len() >= self.batch_size {
            self.flush_buffer()?;
        }
        Ok(())
    }

    /// Flush the buffered touches: batch embed, atomically persist to SQLite, Tantivy, and dense vectors.
    pub fn flush_buffer(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        // 1. Generate batch embeddings for eligible touches (ParseStatus::Ok / Degraded)
        let inputs: Vec<EmbeddingInput> = self
            .buffer
            .iter()
            .map(|t| EmbeddingInput {
                touch_id: t.touch_id,
                text: t.chunk_text.clone(),
                parse_status: t.parse_status,
            })
            .collect();

        let embedding_outputs = self
            .embedder
            .generate_embeddings_batched(&inputs, self.batch_size)?;

        let mut embedding_records = Vec::with_capacity(embedding_outputs.len());
        for out in embedding_outputs {
            let row_offset = self.row_offset_counter;
            self.row_offset_counter += 1;
            self.vector_data.extend_from_slice(&out.embedding);
            embedding_records.push(EmbeddingRecord {
                touch_id: i64::try_from(out.touch_id).unwrap_or(0),
                row_offset,
            });
        }

        // 2. Files, Lineages & Touches in SQLite
        let mut seen_paths = std::collections::HashSet::new();
        let mut file_records = Vec::new();
        for t in &self.buffer {
            if seen_paths.insert(t.path.clone()) {
                file_records.push(FileRecord {
                    id: None,
                    path: t.path.clone(),
                    language: t.language.clone(),
                });
            }
        }
        if !file_records.is_empty() {
            self.sqlite_writer
                .push_files(file_records)
                .map_err(CoreError::InvariantViolation)?;
        }

        let lineages: Vec<LineageRecord> = self
            .buffer
            .iter()
            .map(|t| LineageRecord {
                lineage_id: t.lineage_id,
                current_path: t.path.clone(),
                current_qualified_name: t.qualified_name.clone(),
                language: t.language.clone(),
            })
            .collect();

        self.sqlite_writer
            .push_lineages(lineages)
            .map_err(CoreError::InvariantViolation)?;

        let touches: Vec<TouchRecord> = self
            .buffer
            .iter()
            .map(|t| TouchRecord {
                touch_id: Some(i64::try_from(t.touch_id).unwrap_or(0)),
                lineage_id: t.lineage_id,
                commit_hash: t.commit_hash.clone(),
                path: t.path.clone(),
                qualified_name: t.qualified_name.clone(),
                arity: 0,
                change_type: "modify".to_string(),
                signature_before: None,
                signature_after: Some(t.signature.clone()),
                fingerprint: None,
                chunk_text: t.chunk_text.clone(),
                parse_status: match t.parse_status {
                    ParseStatus::Ok => "ok".to_string(),
                    ParseStatus::Degraded => "degraded".to_string(),
                    ParseStatus::Failed => "failed".to_string(),
                },
                start_line: t.start_line,
                end_line: t.end_line,
            })
            .collect();

        self.sqlite_writer
            .push_touches(touches)
            .map_err(CoreError::InvariantViolation)?;

        if !embedding_records.is_empty() {
            self.sqlite_writer
                .push_embeddings(embedding_records)
                .map_err(CoreError::InvariantViolation)?;
        }

        // 3. Tantivy BM25 indexing
        for t in &self.buffer {
            let doc = TantivyTouchDoc {
                touch_id: t.touch_id,
                lineage_id: t.lineage_id,
                commit_hash: t.commit_hash.clone(),
                qualified_name: t.qualified_name.clone(),
                path: t.path.clone(),
                commit_message: t.commit_message.clone(),
                chunk_text: t.chunk_text.clone(),
            };
            self.tantivy_writer
                .add_touch(&doc)
                .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
        }

        self.buffer.clear();
        Ok(())
    }

    /// Finalize indexing, flush remaining buffers, commit Tantivy, write dense vectors, and return the SQLite connection.
    pub fn finish(mut self, vectors_path: &Path) -> Result<rusqlite::Connection> {
        self.flush_buffer()?;

        self.tantivy_writer
            .finish()
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        write_vector_file_atomic(
            vectors_path,
            &self.vector_data,
            self.dimension as u32,
            self.row_offset_counter,
        )
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        let conn = self
            .sqlite_writer
            .finish()
            .map_err(CoreError::InvariantViolation)?;

        Ok(conn)
    }
}
