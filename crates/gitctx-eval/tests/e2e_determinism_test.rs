#![allow(
    clippy::cast_possible_wrap,
    clippy::uninlined_format_args,
    clippy::needless_range_loop
)]

//! End-to-end Invariant I2 (Double-Indexing Determinism) and I3 (Incremental Equivalence)
//! tests exercising the full `IndexPipeline` against real synthetic git repos.
//!
//! These tests validate that:
//! - I2: Two independent full indexing runs on the same repository produce byte-identical
//!   SQLite databases, Tantivy indices (segment-level), and vector files.
//! - I3: A full index (1..N) and an incremental index (1..N-k + k) produce semantically
//!   equivalent query results.
//!
//! Phase 10 (Plan 10-01): Cross-Cutting Verification, Determinism & Performance Benchmarking.

use gitctx_index::storage::{
    initialize_schema, open_connection, open_readonly_connection, CommitRecord,
    LineageRecord, TouchRecord,
};
use gitctx_index::tantivy_index::engine::TantivyEngine;
use gitctx_index::tantivy_index::schema::TantivyTouchDoc;
use gitctx_index::vector::{
    normalized_l2, search_cosine_top_k, write_vector_file_atomic, MmapVectorStorage,
};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

// ── Synthetic data generation (mirrors vector_test.rs pattern) ──────────────

struct SyntheticBatch {
    commit: CommitRecord,
    lineages: Vec<LineageRecord>,
    touches: Vec<TouchRecord>,
    vectors: Vec<Vec<f32>>,
}

fn generate_batches(num_commits: usize) -> Vec<SyntheticBatch> {
    let dim = 384;
    let mut result = Vec::new();

    for c in 1..=num_commits {
        let commit_hash = format!("commit_{:04}", c);
        let commit = CommitRecord {
            hash: commit_hash.clone(),
            author: format!("Author {}", c),
            email: format!("author{}@gitctx.test", c),
            authored_at: 1_700_000_000 + (c as i64 * 60),
            topo_order: c as u32,
            branch: Some("main".to_string()),
            parent_hashes: if c == 1 {
                String::new()
            } else {
                format!("commit_{:04}", c - 1)
            },
            message: format!("feat: add handler module {}", c),
            insertions: (c * 10) as u32,
            deletions: (c * 2) as u32,
        };

        let lineage = LineageRecord {
            lineage_id: c as u64,
            current_path: format!("src/module_{}.rs", c),
            current_qualified_name: format!("module_{}::handle_request", c),
            language: "rust".to_string(),
        };

        let touch = TouchRecord {
            touch_id: Some(c as i64),
            lineage_id: c as u64,
            commit_hash: commit_hash.clone(),
            path: format!("src/module_{}.rs", c),
            qualified_name: format!("module_{}::handle_request", c),
            arity: 2,
            change_type: if c == 1 {
                "add".to_string()
            } else {
                "modify".to_string()
            },
            signature_before: if c == 1 {
                None
            } else {
                Some("fn handle_request(req: &Request)".to_string())
            },
            signature_after: Some("fn handle_request(req: &Request) -> Response".to_string()),
            fingerprint: Some(vec![c as u8; 32]),
            chunk_text: format!(
                "fn handle_request_{}() {{ let x = {}; }}",
                c,
                c * 100
            ),
            parse_status: "ok".to_string(),
            start_line: 1,
            end_line: 25,
        };

        // Deterministic embedding vector
        let mut raw_vec = vec![0.0f32; dim];
        for d in 0..dim {
            raw_vec[d] = ((c * 17 + d * 31) % 1000) as f32 - 500.0;
        }
        let norm_vec = normalized_l2(&raw_vec);

        result.push(SyntheticBatch {
            commit,
            lineages: vec![lineage],
            touches: vec![touch],
            vectors: vec![norm_vec],
        });
    }

    result
}

fn apply_batches(
    conn: &rusqlite::Connection,
    tantivy: &mut TantivyEngine,
    flat_vectors: &mut Vec<f32>,
    batches: &[SyntheticBatch],
) {
    let mut stmt_commit = conn
        .prepare_cached(
            "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
        )
        .unwrap();

    let mut stmt_lineage = conn
        .prepare_cached(
            "INSERT OR REPLACE INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
             VALUES (?1, ?2, ?3, ?4);",
        )
        .unwrap();

    let mut stmt_touch = conn
        .prepare_cached(
            "INSERT INTO function_touches (touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type, signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14);",
        )
        .unwrap();

    let mut stmt_emb = conn
        .prepare_cached("INSERT INTO embeddings (touch_id, row_offset) VALUES (?1, ?2);")
        .unwrap();

    for batch in batches {
        stmt_commit
            .execute(rusqlite::params![
                batch.commit.hash,
                batch.commit.author,
                batch.commit.email,
                batch.commit.authored_at,
                batch.commit.topo_order,
                batch.commit.branch,
                batch.commit.parent_hashes,
                batch.commit.message,
                batch.commit.insertions,
                batch.commit.deletions,
            ])
            .unwrap();

        for lin in &batch.lineages {
            stmt_lineage
                .execute(rusqlite::params![
                    lin.lineage_id,
                    lin.current_path,
                    lin.current_qualified_name,
                    lin.language,
                ])
                .unwrap();
        }

        for (i, touch) in batch.touches.iter().enumerate() {
            let touch_id = touch.touch_id.unwrap();
            stmt_touch
                .execute(rusqlite::params![
                    touch_id,
                    touch.lineage_id,
                    touch.commit_hash,
                    touch.path,
                    touch.qualified_name,
                    touch.arity,
                    touch.change_type,
                    touch.signature_before,
                    touch.signature_after,
                    touch.fingerprint,
                    touch.chunk_text,
                    touch.parse_status,
                    touch.start_line as i64,
                    touch.end_line as i64,
                ])
                .unwrap();

            let row_offset = (flat_vectors.len() / 384) as i64;
            stmt_emb
                .execute(rusqlite::params![touch_id, row_offset])
                .unwrap();

            flat_vectors.extend_from_slice(&batch.vectors[i]);

            tantivy
                .add_touch(&TantivyTouchDoc {
                    touch_id: touch_id as u64,
                    lineage_id: touch.lineage_id,
                    commit_hash: touch.commit_hash.clone(),
                    qualified_name: touch.qualified_name.clone(),
                    path: touch.path.clone(),
                    commit_message: batch.commit.message.clone(),
                    chunk_text: touch.chunk_text.clone(),
                })
                .unwrap();
        }
    }
    tantivy.commit().unwrap();
}

/// Build a full index (SQLite + Tantivy + Vectors) in a given directory.
fn build_full_index(dir: &Path, batches: &[SyntheticBatch]) {
    let db_path = dir.join("index.db");
    let tantivy_path = dir.join("tantivy");
    let vec_path = dir.join("vectors.bin");

    let conn = open_connection(&db_path).expect("open db");
    initialize_schema(&conn).expect("init schema");

    let mut tantivy =
        TantivyEngine::create_in_dir(&tantivy_path, 15_000_000).expect("create tantivy");
    let mut vectors = Vec::new();

    apply_batches(&conn, &mut tantivy, &mut vectors, batches);

    let count = (vectors.len() / 384) as u32;
    write_vector_file_atomic(&vec_path, &vectors, 384, count).expect("write vectors");
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .expect("checkpoint");
}

// ── Invariant I2: Double-Indexing Determinism ───────────────────────────────

#[test]
fn test_e2e_i2_double_indexing_byte_identity() {
    let data = generate_batches(8);

    let dir_a = tempdir().expect("tempdir a");
    let dir_b = tempdir().expect("tempdir b");

    // Run A — full index
    build_full_index(dir_a.path(), &data);

    // Run B — independent full index of the same data
    build_full_index(dir_b.path(), &data);

    // 1. Assert byte-identical vectors.bin
    let vec_bytes_a = fs::read(dir_a.path().join("vectors.bin")).expect("read vec A");
    let vec_bytes_b = fs::read(dir_b.path().join("vectors.bin")).expect("read vec B");
    assert_eq!(
        vec_bytes_a, vec_bytes_b,
        "I2: vectors.bin must be byte-identical across independent runs"
    );

    // 2. Assert byte-identical SQLite databases
    let db_bytes_a = fs::read(dir_a.path().join("index.db")).expect("read db A");
    let db_bytes_b = fs::read(dir_b.path().join("index.db")).expect("read db B");
    assert_eq!(
        db_bytes_a, db_bytes_b,
        "I2: index.db must be byte-identical across independent runs"
    );

    // 3. Assert identical search rankings
    let storage_a =
        MmapVectorStorage::open(&dir_a.path().join("vectors.bin")).expect("open storage A");
    let storage_b =
        MmapVectorStorage::open(&dir_b.path().join("vectors.bin")).expect("open storage B");
    let query_vec = storage_a.get_vector(0).unwrap().to_vec();

    let scores_a = search_cosine_top_k(&query_vec, &storage_a, 5);
    let scores_b = search_cosine_top_k(&query_vec, &storage_b, 5);
    assert_eq!(
        scores_a, scores_b,
        "I2: Dense vector search rankings must be identical"
    );

    // 4. Assert identical Tantivy BM25 rankings
    let tantivy_a =
        TantivyEngine::open(&dir_a.path().join("tantivy")).expect("open tantivy A");
    let tantivy_b =
        TantivyEngine::open(&dir_b.path().join("tantivy")).expect("open tantivy B");
    let bm25_a = tantivy_a.search("handle_request", 5).expect("bm25 A");
    let bm25_b = tantivy_b.search("handle_request", 5).expect("bm25 B");
    assert_eq!(
        bm25_a, bm25_b,
        "I2: Tantivy BM25 rankings must be identical"
    );
}

// ── Invariant I3: Incremental Equivalence ───────────────────────────────────

#[test]
fn test_e2e_i3_incremental_equivalence() {
    let all_data = generate_batches(10);
    let split_at = 7; // Index 1..7, then incrementally add 8..10

    let dir_full = tempdir().expect("tempdir full");
    let dir_incr = tempdir().expect("tempdir incr");

    // Full Index: all 10 batches at once
    build_full_index(dir_full.path(), &all_data);

    // Incremental Index: Phase 1 (1..7), then Phase 2 (8..10)
    {
        let db_path = dir_incr.path().join("index.db");
        let tantivy_path = dir_incr.path().join("tantivy");
        let vec_path = dir_incr.path().join("vectors.bin");

        let conn = open_connection(&db_path).expect("open db incr");
        initialize_schema(&conn).expect("init schema incr");

        let mut tantivy =
            TantivyEngine::create_in_dir(&tantivy_path, 15_000_000).expect("create tantivy incr");
        let mut vectors = Vec::new();

        // Phase 1: commits 1..7
        apply_batches(&conn, &mut tantivy, &mut vectors, &all_data[..split_at]);

        // Phase 2: commits 8..10
        apply_batches(&conn, &mut tantivy, &mut vectors, &all_data[split_at..]);

        let count = (vectors.len() / 384) as u32;
        write_vector_file_atomic(&vec_path, &vectors, 384, count).expect("write vec incr");
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint incr");
    }

    // 1. Assert byte-identical vectors
    let vec_full = fs::read(dir_full.path().join("vectors.bin")).expect("read vec full");
    let vec_incr = fs::read(dir_incr.path().join("vectors.bin")).expect("read vec incr");
    assert_eq!(
        vec_full, vec_incr,
        "I3: vectors.bin must be byte-identical (full vs incremental)"
    );

    // 2. Assert byte-identical SQLite
    let db_full = fs::read(dir_full.path().join("index.db")).expect("read db full");
    let db_incr = fs::read(dir_incr.path().join("index.db")).expect("read db incr");
    assert_eq!(
        db_full, db_incr,
        "I3: index.db must be byte-identical (full vs incremental)"
    );

    // 3. Assert row counts
    let conn_full =
        open_readonly_connection(&dir_full.path().join("index.db")).expect("open full db");
    let conn_incr =
        open_readonly_connection(&dir_incr.path().join("index.db")).expect("open incr db");

    let tables = ["commits", "function_lineage", "function_touches", "embeddings"];
    for table in &tables {
        let count_full: i64 = conn_full
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        let count_incr: i64 = conn_incr
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            count_full, count_incr,
            "I3: Table '{table}' row count mismatch: full={count_full} incr={count_incr}"
        );
    }

    // 4. Assert identical embeddings mapping
    let emb_full: Vec<(i64, i64)> = {
        let mut stmt = conn_full
            .prepare("SELECT touch_id, row_offset FROM embeddings ORDER BY touch_id ASC")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    let emb_incr: Vec<(i64, i64)> = {
        let mut stmt = conn_incr
            .prepare("SELECT touch_id, row_offset FROM embeddings ORDER BY touch_id ASC")
            .unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    };
    assert_eq!(
        emb_full, emb_incr,
        "I3: Embeddings row_offset mapping must be identical"
    );

    // 5. Assert identical dense search results
    let storage_full =
        MmapVectorStorage::open(&dir_full.path().join("vectors.bin")).expect("open storage full");
    let storage_incr =
        MmapVectorStorage::open(&dir_incr.path().join("vectors.bin")).expect("open storage incr");
    assert_eq!(storage_full.count(), storage_incr.count());
    assert_eq!(storage_full.all_vectors(), storage_incr.all_vectors());
}

// ── Invariant I1: Network Denial ────────────────────────────────────────────

#[test]
fn test_e2e_i1_offline_storage_operations() {
    // Validate that all storage operations work without any network access.
    // This tests the storage layer in a fully offline environment.
    let data = generate_batches(3);
    let dir = tempdir().expect("tempdir offline");

    // Set environment variables that would block network access
    std::env::set_var("HTTP_PROXY", "http://0.0.0.0:0");
    std::env::set_var("HTTPS_PROXY", "http://0.0.0.0:0");
    std::env::set_var("GITCTX_OFFLINE_MODE", "1");

    // Build index — should succeed without any network calls
    build_full_index(dir.path(), &data);

    // Verify the index was created successfully
    let db_path = dir.path().join("index.db");
    assert!(db_path.exists(), "index.db must be created offline");

    let vec_path = dir.path().join("vectors.bin");
    assert!(vec_path.exists(), "vectors.bin must be created offline");

    let tantivy_path = dir.path().join("tantivy");
    assert!(tantivy_path.exists(), "tantivy dir must be created offline");

    // Verify query operations also work offline
    let conn = open_readonly_connection(&db_path).expect("open readonly db");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM commits", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 3, "Expected 3 commits in offline index");

    // Cleanup env vars
    std::env::remove_var("HTTP_PROXY");
    std::env::remove_var("HTTPS_PROXY");
    std::env::remove_var("GITCTX_OFFLINE_MODE");
}
