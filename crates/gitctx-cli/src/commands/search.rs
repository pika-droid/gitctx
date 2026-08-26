use crate::envelope::ResponseEnvelope;
use crate::format::format_search;
use crate::Format;
use anyhow::{Context, Result};
use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{StorageEngine, CURRENT_SCHEMA_VERSION};
use gitctx_index::tantivy_index::TantivyEngine;
use gitctx_index::vector::{FastEmbedGenerator, MmapVectorStorage};
use gitctx_query::retrieval::HybridRetriever;
use std::path::Path;
use std::time::Instant;

pub fn run(query: &str, format: Format) -> Result<()> {
    let start_time = Instant::now();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let db_path = current_dir.join(".gitctx").join("index.db");
    let tantivy_path = current_dir.join(".gitctx").join("tantivy");
    let vectors_path = current_dir.join(".gitctx").join("vectors.bin");

    if !db_path.exists() {
        anyhow::bail!(
            "Index database not found at {}. Have you run 'gitctx index'?",
            db_path.display()
        );
    }

    let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "BGESmallENV15", "code", 500, 1);

    let storage =
        StorageEngine::open(&db_path, &config).context("Failed to open storage engine")?;

    let tantivy = TantivyEngine::open(&tantivy_path).ok();
    let vectors = MmapVectorStorage::open(&vectors_path).ok();

    // Embed the query for dense vector search if vectors exist
    let query_vector = if vectors.is_some() {
        let mut embedder =
            FastEmbedGenerator::new(None).context("Failed to initialize embedder")?;
        let mut embeddings = embedder
            .embed_texts(&[query.to_string()], None)
            .context("Failed to embed query")?;
        embeddings.pop()
    } else {
        None
    };

    let retriever = HybridRetriever::new(storage.connection(), tantivy.as_ref(), vectors.as_ref());

    let results = retriever
        .search(query, query_vector.as_deref(), 10)
        .context("Search failed")?;

    let duration = start_time.elapsed();

    if format == Format::Json {
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": "success",
            "query": query,
            "results": results,
            "duration_ms": duration.as_millis(),
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        format_search(query, &results, duration);
    }

    Ok(())
}
