use crate::tantivy_index::schema::{
    TantivySchema, TantivyTouchDoc, BOOST_CHUNK_TEXT, BOOST_COMMIT_MESSAGE, BOOST_PATH,
    BOOST_QUALIFIED_NAME, CODE_TOKENIZER_NAME,
};
use crate::tantivy_index::tokenizer::CodeTokenizer;
use std::fs;
use std::path::{Path, PathBuf};
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::{Index, IndexReader, IndexWriter, TantivyDocument};

/// Lexical search engine powered by Tantivy BM25 with code-aware tokenization and boosting.
pub struct TantivyEngine {
    index: Index,
    schema_fields: TantivySchema,
    reader: IndexReader,
    writer: Option<IndexWriter>,
    index_path: Option<PathBuf>,
}

impl TantivyEngine {
    /// Initialize Tantivy engine in a local directory (e.g. `.gitctx/tantivy.tmp/` or `.gitctx/tantivy/`).
    pub fn create_in_dir<P: AsRef<Path>>(dir_path: P, memory_budget_bytes: usize) -> Result<Self, tantivy::TantivyError> {
        let path = dir_path.as_ref().to_path_buf();
        fs::create_dir_all(&path).map_err(|e| tantivy::TantivyError::SystemError(e.to_string()))?;

        let schema_fields = TantivySchema::new();
        let mmap_dir = MmapDirectory::open(&path)?;
        let index = Index::open_or_create(mmap_dir, schema_fields.schema.clone())?;

        // Register custom CodeTokenizer
        index
            .tokenizers()
            .register(CODE_TOKENIZER_NAME, CodeTokenizer);

        let reader = index.reader()?;
        let writer = index.writer(memory_budget_bytes)?;

        Ok(Self {
            index,
            schema_fields,
            reader,
            writer: Some(writer),
            index_path: Some(path),
        })
    }

    /// Open an existing Tantivy index directory for reading/querying.
    pub fn open<P: AsRef<Path>>(dir_path: P) -> Result<Self, tantivy::TantivyError> {
        let path = dir_path.as_ref().to_path_buf();
        let schema_fields = TantivySchema::new();
        let mmap_dir = MmapDirectory::open(&path)?;
        let index = Index::open(mmap_dir)?;

        index
            .tokenizers()
            .register(CODE_TOKENIZER_NAME, CodeTokenizer);

        let reader = index.reader()?;

        Ok(Self {
            index,
            schema_fields,
            reader,
            writer: None,
            index_path: Some(path),
        })
    }

    /// Create an in-memory Tantivy engine for fast unit and integration testing.
    pub fn create_in_ram() -> Result<Self, tantivy::TantivyError> {
        let schema_fields = TantivySchema::new();
        let index = Index::create_in_ram(schema_fields.schema.clone());

        index
            .tokenizers()
            .register(CODE_TOKENIZER_NAME, CodeTokenizer);

        let reader = index.reader()?;
        let writer = index.writer(32_000_000)?;

        Ok(Self {
            index,
            schema_fields,
            reader,
            writer: Some(writer),
            index_path: None,
        })
    }

    pub fn schema_fields(&self) -> &TantivySchema {
        &self.schema_fields
    }

    pub fn index_path(&self) -> Option<&Path> {
        self.index_path.as_deref()
    }

    /// Add a single function touch document to the index writer.
    pub fn add_touch(&self, doc: &TantivyTouchDoc) -> Result<(), tantivy::TantivyError> {
        if let Some(writer) = &self.writer {
            let mut tantivy_doc = TantivyDocument::default();
            tantivy_doc.add_u64(self.schema_fields.touch_id, doc.touch_id);
            tantivy_doc.add_u64(self.schema_fields.lineage_id, doc.lineage_id);
            tantivy_doc.add_text(self.schema_fields.commit_hash, &doc.commit_hash);
            tantivy_doc.add_text(self.schema_fields.qualified_name, &doc.qualified_name);
            tantivy_doc.add_text(self.schema_fields.path, &doc.path);
            tantivy_doc.add_text(self.schema_fields.commit_message, &doc.commit_message);
            tantivy_doc.add_text(self.schema_fields.chunk_text, &doc.chunk_text);

            writer.add_document(tantivy_doc)?;
            Ok(())
        } else {
            Err(tantivy::TantivyError::SystemError(
                "Cannot add document to read-only TantivyEngine".to_string(),
            ))
        }
    }

    /// Add a batch of function touches to the index writer.
    pub fn add_touches(&self, docs: &[TantivyTouchDoc]) -> Result<(), tantivy::TantivyError> {
        if let Some(writer) = &self.writer {
            for doc in docs {
                let mut tantivy_doc = TantivyDocument::default();
                tantivy_doc.add_u64(self.schema_fields.touch_id, doc.touch_id);
                tantivy_doc.add_u64(self.schema_fields.lineage_id, doc.lineage_id);
                tantivy_doc.add_text(self.schema_fields.commit_hash, &doc.commit_hash);
                tantivy_doc.add_text(self.schema_fields.qualified_name, &doc.qualified_name);
                tantivy_doc.add_text(self.schema_fields.path, &doc.path);
                tantivy_doc.add_text(self.schema_fields.commit_message, &doc.commit_message);
                tantivy_doc.add_text(self.schema_fields.chunk_text, &doc.chunk_text);
                writer.add_document(tantivy_doc)?;
            }
            Ok(())
        } else {
            Err(tantivy::TantivyError::SystemError(
                "Cannot add documents to read-only TantivyEngine".to_string(),
            ))
        }
    }

    /// Commit uncommitted documents in the index writer and reload index reader.
    pub fn commit(&mut self) -> Result<(), tantivy::TantivyError> {
        if let Some(writer) = &mut self.writer {
            writer.commit()?;
            self.reader.reload()?;
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Commit pending changes, close writer, and atomically swap directory to `target_dir` per D-07.
    pub fn commit_and_swap_to<P: AsRef<Path>>(&mut self, target_dir: P) -> std::io::Result<()> {
        if let Some(mut writer) = self.writer.take() {
            let _ = writer.commit();
            let _ = writer.wait_merging_threads();
        }

        if let Some(src_path) = &self.index_path {
            let target = target_dir.as_ref();
            if target.exists() {
                fs::remove_dir_all(target)?;
            }
            fs::rename(src_path, target)?;
            self.index_path = Some(target.to_path_buf());
        }

        Ok(())
    }

    /// Search the Tantivy BM25 index with boosted fields per D-06 and INDEX-03.
    ///
    /// Multi-field boost weights:
    /// - `qualified_name`: 4.0
    /// - `path`: 2.0
    /// - `commit_message`: 1.2
    /// - `chunk_text`: 1.0
    pub fn search(&self, query_str: &str, top_k: usize) -> Result<Vec<(u64, f32)>, tantivy::TantivyError> {
        let searcher = self.reader.searcher();

        let search_fields = vec![
            self.schema_fields.qualified_name,
            self.schema_fields.path,
            self.schema_fields.commit_message,
            self.schema_fields.chunk_text,
        ];

        let mut query_parser = QueryParser::for_index(&self.index, search_fields);
        query_parser.set_field_boost(self.schema_fields.qualified_name, BOOST_QUALIFIED_NAME);
        query_parser.set_field_boost(self.schema_fields.path, BOOST_PATH);
        query_parser.set_field_boost(self.schema_fields.commit_message, BOOST_COMMIT_MESSAGE);
        query_parser.set_field_boost(self.schema_fields.chunk_text, BOOST_CHUNK_TEXT);

        // Sanitize or fallback parse
        let parsed_query = match query_parser.parse_query(query_str) {
            Ok(q) => q,
            Err(_) => {
                // If direct parse fails (due to syntax characters in code snippet), sanitize to terms
                let sanitized: String = query_str
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '_' { c } else { ' ' })
                    .collect();
                let clean = sanitized.split_whitespace().collect::<Vec<_>>().join(" ");
                if clean.is_empty() {
                    return Ok(Vec::new());
                }
                query_parser.parse_query(&clean).unwrap_or_else(|_| {
                    Box::new(tantivy::query::EmptyQuery)
                })
            }
        };

        let top_docs = searcher.search(&parsed_query, &TopDocs::with_limit(top_k))?;
        let mut results = Vec::with_capacity(top_docs.len());

        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            if let Some(val) = doc.get_first(self.schema_fields.touch_id) {
                if let Some(touch_id) = val.as_u64() {
                    results.push((touch_id, score));
                }
            }
        }

        Ok(results)
    }
}
