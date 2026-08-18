use crate::citation::CitationBuilder;
use crate::rrf::{fuse_rankings, RrfParams};
use crate::symbol::ExactSymbolMatcher;
use gitctx_core::error::CoreError;
use gitctx_core::model::Citation;
use gitctx_index::storage::TouchRecord;
use gitctx_index::tantivy_index::TantivyEngine;
use gitctx_index::vector::{search_cosine_top_k, MmapVectorStorage};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Maximum number of touches retained per commit in hybrid search results per D-15 and INDEX-05.
pub const DEFAULT_MAX_TOUCHES_PER_COMMIT: usize = 3;

/// Search result fact with complete structured citation metadata per INDEX-07 and Invariant I4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievedFact {
    pub touch_id: u64,
    pub lineage_id: u64,
    pub commit_hash: String,
    pub path: String,
    pub qualified_name: String,
    pub change_type: String,
    pub chunk_text: String,
    pub score: f64,
    pub rank: usize,
    pub citation: Citation,
}

/// Deduplicate candidate touches by BLAKE3 chunk text hash and cap touches per commit hash per D-15 and INDEX-05.
pub fn deduplicate_and_cap_touches(
    candidates: Vec<(TouchRecord, f64, String, f32)>,
    max_touches_per_commit: usize,
    limit: usize,
) -> Result<Vec<RetrievedFact>, CoreError> {
    let mut seen_chunk_hashes: HashSet<blake3::Hash> = HashSet::new();
    let mut commit_touch_counts: HashMap<String, usize> = HashMap::new();
    let mut results = Vec::new();

    for (touch, score, match_rule, match_confidence) in candidates {
        if results.len() >= limit {
            break;
        }

        // 1. Chunk Text Hash Deduplication
        let chunk_hash = blake3::hash(touch.chunk_text.as_bytes());
        if !seen_chunk_hashes.insert(chunk_hash) {
            // Duplicate chunk text encountered; skip to preserve highest-scoring candidate
            continue;
        }

        // 2. Commit Density Capping
        let count = commit_touch_counts.entry(touch.commit_hash.clone()).or_insert(0);
        if *count >= max_touches_per_commit {
            // Maximum touch threshold reached for this commit; skip candidate
            continue;
        }
        *count += 1;

        // 3. Attach structured Citation
        let citation = CitationBuilder::from_touch(&touch, &match_rule, match_confidence)?;

        let rank = results.len();
        results.push(RetrievedFact {
            touch_id: touch.touch_id.unwrap_or(0) as u64,
            lineage_id: touch.lineage_id,
            commit_hash: touch.commit_hash,
            path: touch.path,
            qualified_name: touch.qualified_name,
            change_type: touch.change_type,
            chunk_text: touch.chunk_text,
            score,
            rank,
            citation,
        });
    }

    Ok(results)
}

/// Hybrid retrieval engine coordinating Exact Symbol Fast-Path, Tantivy BM25, and flat SIMD dense vector search.
pub struct HybridRetriever<'a> {
    conn: &'a Connection,
    tantivy: Option<&'a TantivyEngine>,
    vectors: Option<&'a MmapVectorStorage>,
    rrf_params: RrfParams,
    max_touches_per_commit: usize,
}

impl<'a> HybridRetriever<'a> {
    pub fn new(
        conn: &'a Connection,
        tantivy: Option<&'a TantivyEngine>,
        vectors: Option<&'a MmapVectorStorage>,
    ) -> Self {
        Self {
            conn,
            tantivy,
            vectors,
            rrf_params: RrfParams::default(),
            max_touches_per_commit: DEFAULT_MAX_TOUCHES_PER_COMMIT,
        }
    }

    pub fn with_rrf_params(mut self, params: RrfParams) -> Self {
        self.rrf_params = params;
        self
    }

    pub fn with_max_touches_per_commit(mut self, max: usize) -> Self {
        self.max_touches_per_commit = max;
        self
    }

    /// Query the database for a single TouchRecord by touch_id.
    pub fn fetch_touch(&self, touch_id: u64) -> rusqlite::Result<Option<TouchRecord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT 
                touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type,
                signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line
            FROM function_touches
            WHERE touch_id = ?1;",
        )?;

        let mut rows = stmt.query(rusqlite::params![touch_id as i64])?;
        if let Some(row) = rows.next()? {
            let start_line_i64: i64 = row.get(12)?;
            let end_line_i64: i64 = row.get(13)?;
            Ok(Some(TouchRecord {
                touch_id: row.get(0)?,
                lineage_id: row.get(1)?,
                commit_hash: row.get(2)?,
                path: row.get(3)?,
                qualified_name: row.get(4)?,
                arity: row.get(5)?,
                change_type: row.get(6)?,
                signature_before: row.get(7)?,
                signature_after: row.get(8)?,
                fingerprint: row.get(9)?,
                chunk_text: row.get(10)?,
                parse_status: row.get(11)?,
                start_line: start_line_i64 as usize,
                end_line: end_line_i64 as usize,
            }))
        } else {
            Ok(None)
        }
    }

    /// Build vector row_offset to touch_id mapping from SQLite embeddings table.
    fn load_row_offset_to_touch_map(&self) -> rusqlite::Result<HashMap<usize, u64>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT touch_id, row_offset FROM embeddings;",
        )?;
        let rows = stmt.query_map([], |row| {
            let touch_id: i64 = row.get(0)?;
            let row_offset: u32 = row.get(1)?;
            Ok((row_offset as usize, touch_id as u64))
        })?;

        let mut map = HashMap::new();
        for r in rows {
            let (offset, id) = r?;
            map.insert(offset, id);
        }
        Ok(map)
    }

    /// Execute unified hybrid search query per D-13, D-14, D-15, INDEX-05, INDEX-07.
    pub fn search(
        &self,
        query: &str,
        query_vector: Option<&[f32]>,
        limit: usize,
    ) -> Result<Vec<RetrievedFact>, CoreError> {
        let candidate_pool_size = (limit * 5).max(50);

        // 1. Exact Symbol Fast-Path (D-13)
        let exact_matches = ExactSymbolMatcher::find_exact_symbol_touches(
            self.conn,
            query,
            candidate_pool_size,
        ).map_err(|e| CoreError::InvariantViolation(format!("Exact symbol query error: {e}")))?;

        // 2. Lexical Tantivy BM25 Search
        let bm25_hits: Vec<u64> = if let Some(tantivy) = self.tantivy {
            tantivy
                .search(query, candidate_pool_size)
                .map_err(|e| CoreError::InvariantViolation(format!("Tantivy search error: {e}")))?
                .into_iter()
                .map(|(touch_id, _score)| touch_id)
                .collect()
        } else {
            Vec::new()
        };

        // 3. Dense Vector SIMD Cosine Search
        let dense_hits: Vec<u64> = if let (Some(vectors), Some(q_vec)) = (self.vectors, query_vector) {
            let offset_to_id = self.load_row_offset_to_touch_map()
                .map_err(|e| CoreError::InvariantViolation(format!("Embeddings map query error: {e}")))?;
            let top_vectors = search_cosine_top_k(q_vec, vectors, candidate_pool_size);
            top_vectors
                .into_iter()
                .filter_map(|(offset, _score)| offset_to_id.get(&offset).copied())
                .collect()
        } else {
            Vec::new()
        };

        // 4. Reciprocal Rank Fusion (D-14)
        let fused_candidates = fuse_rankings(
            &exact_matches,
            &bm25_hits,
            &dense_hits,
            &self.rrf_params,
        );

        // 5. Hydrate candidates from SQLite
        let exact_set: HashSet<u64> = exact_matches.into_iter().collect();
        let mut candidates_to_filter = Vec::new();

        for candidate in fused_candidates {
            if let Some(touch) = self.fetch_touch(candidate.touch_id)
                .map_err(|e| CoreError::InvariantViolation(format!("Touch fetch error: {e}")))?
            {
                let is_exact = exact_set.contains(&candidate.touch_id);
                let match_rule = if is_exact {
                    "exact_symbol".to_string()
                } else if !bm25_hits.is_empty() && !dense_hits.is_empty() {
                    "rrf_bm25_dense".to_string()
                } else if !bm25_hits.is_empty() {
                    "bm25".to_string()
                } else {
                    "dense_vector".to_string()
                };

                let match_confidence = if is_exact {
                    1.0f32
                } else {
                    // Normalized proxy confidence based on RRF score
                    (candidate.score * 60.0).clamp(0.0, 1.0) as f32
                };

                candidates_to_filter.push((touch, candidate.score, match_rule, match_confidence));
            }
        }

        // 6. Deduplication and Commit Density Capping (D-15)
        deduplicate_and_cap_touches(
            candidates_to_filter,
            self.max_touches_per_commit,
            limit,
        )
    }
}
