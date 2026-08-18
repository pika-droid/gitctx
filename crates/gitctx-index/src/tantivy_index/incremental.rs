use crate::tantivy_index::schema::{TantivySchema, TantivyTouchDoc, CODE_TOKENIZER_NAME};
use crate::tantivy_index::tokenizer::CodeTokenizer;
use std::path::{Path, PathBuf};
use tantivy::directory::MmapDirectory;
use tantivy::{Index, IndexWriter, TantivyDocument};

pub const DEFAULT_INCREMENTAL_MEMORY_BUDGET_BYTES: usize = 32_000_000; // 32MB budget

/// Fast incremental Tantivy index writer for appending document touches per D-08 and INDEX-03.
pub struct IncrementalTantivyWriter {
    index: Index,
    schema_fields: TantivySchema,
    writer: IndexWriter,
    index_path: PathBuf,
}

impl IncrementalTantivyWriter {
    /// Open existing Tantivy index directory at `dir_path` for incremental document appending.
    pub fn open<P: AsRef<Path>>(dir_path: P) -> Result<Self, tantivy::TantivyError> {
        Self::open_with_budget(dir_path, DEFAULT_INCREMENTAL_MEMORY_BUDGET_BYTES)
    }

    /// Open existing Tantivy index directory with a custom memory budget in bytes.
    pub fn open_with_budget<P: AsRef<Path>>(
        dir_path: P,
        memory_budget_bytes: usize,
    ) -> Result<Self, tantivy::TantivyError> {
        let path = dir_path.as_ref().to_path_buf();
        let schema_fields = TantivySchema::new();
        let mmap_dir = MmapDirectory::open(&path)?;
        let index = Index::open_or_create(mmap_dir, schema_fields.schema.clone())?;

        index
            .tokenizers()
            .register(CODE_TOKENIZER_NAME, CodeTokenizer);

        let writer = index.writer(memory_budget_bytes)?;

        Ok(Self {
            index,
            schema_fields,
            writer,
            index_path: path,
        })
    }

    pub fn schema_fields(&self) -> &TantivySchema {
        &self.schema_fields
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn index_path(&self) -> &Path {
        &self.index_path
    }

    /// Append a new function touch document into active segments without rebuilding.
    pub fn add_touch(&mut self, doc: &TantivyTouchDoc) -> Result<(), tantivy::TantivyError> {
        let mut tantivy_doc = TantivyDocument::default();
        tantivy_doc.add_u64(self.schema_fields.touch_id, doc.touch_id);
        tantivy_doc.add_u64(self.schema_fields.lineage_id, doc.lineage_id);
        tantivy_doc.add_text(self.schema_fields.commit_hash, &doc.commit_hash);
        tantivy_doc.add_text(self.schema_fields.qualified_name, &doc.qualified_name);
        tantivy_doc.add_text(self.schema_fields.path, &doc.path);
        tantivy_doc.add_text(self.schema_fields.commit_message, &doc.commit_message);
        tantivy_doc.add_text(self.schema_fields.chunk_text, &doc.chunk_text);

        self.writer.add_document(tantivy_doc)?;
        Ok(())
    }

    /// Append a batch of function touch documents into active segments.
    pub fn add_touches(&mut self, docs: &[TantivyTouchDoc]) -> Result<(), tantivy::TantivyError> {
        for doc in docs {
            self.add_touch(doc)?;
        }
        Ok(())
    }

    /// Commit newly added documents in <2s, letting Tantivy handle segment merges in the background.
    pub fn commit(&mut self) -> Result<u64, tantivy::TantivyError> {
        let opstamp = self.writer.commit()?;
        Ok(opstamp)
    }

    /// Complete incremental write and wait for background segment merging threads.
    pub fn finish(mut self) -> Result<u64, tantivy::TantivyError> {
        let opstamp = self.writer.commit()?;
        self.writer.wait_merging_threads()?;
        Ok(opstamp)
    }
}
