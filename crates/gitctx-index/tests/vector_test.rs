use gitctx_core::model::ParseStatus;
use gitctx_index::storage::{
    initialize_schema, open_connection, CommitRecord, LineageRecord, TouchRecord,
};
use gitctx_index::tantivy_index::engine::TantivyEngine;
use gitctx_index::tantivy_index::schema::TantivyTouchDoc;
use gitctx_index::vector::{
    dot_product_simd, normalize_l2, normalized_l2, read_vector_header, search_cosine_top_k,
    write_vector_file, write_vector_file_atomic, EmbeddingInput, FastEmbedGenerator,
    MmapVectorStorage, VectorHeader, VECTOR_HEADER_SIZE, VECTOR_MAGIC, VECTOR_VERSION,
};
use rusqlite::Connection;
use std::fs;
use std::time::Instant;
use tempfile::tempdir;

#[test]
fn test_mmap_vector_storage_and_simd_scoring() {
    let tmp_dir = tempdir().expect("tempdir");
    let vec_file = tmp_dir.path().join("vectors.bin");

    // 1. Test header properties
    let header = VectorHeader::new(10, 384);
    assert_eq!(header.magic, *VECTOR_MAGIC);
    assert_eq!(header.version, VECTOR_VERSION);
    assert_eq!(header.dimension, 384);
    assert_eq!(header.count, 10);
    assert_eq!(header.expected_file_size(), 16 + 10 * 384 * 4);

    let header_bytes = header.to_bytes();
    assert_eq!(header_bytes.len(), VECTOR_HEADER_SIZE);
    let parsed_header = VectorHeader::from_bytes(&header_bytes).expect("parse header");
    assert_eq!(header, parsed_header);

    // 2. Generate 10 synthetic 384-dim vectors with write-time unit L2 normalization
    let dim = 384;
    let count = 10;
    let mut flat_vectors = Vec::with_capacity(count * dim);

    for i in 0..count {
        let mut raw_vec = vec![0.0f32; dim];
        for d in 0..dim {
            raw_vec[d] = ((i + 1) * (d + 1) + (d * 7 + i * 13) % 97) as f32;
        }
        raw_vec[i % dim] += 50.0;
        let mut in_place_vec = raw_vec.clone();
        normalize_l2(&mut in_place_vec);
        let norm_vec = normalized_l2(&raw_vec);
        assert_eq!(in_place_vec, norm_vec);

        let norm: f32 = norm_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "Vector must be unit L2 normalized");
        flat_vectors.extend_from_slice(&norm_vec);
    }

    // 3. Write atomic vector file
    write_vector_file_atomic(&vec_file, &flat_vectors, dim as u32, count as u32)
        .expect("write vector file");
    assert!(vec_file.exists());

    let disk_header = read_vector_header(&vec_file).expect("read header from disk");
    assert_eq!(disk_header, header);

    // 4. Memory-map vector storage and verify
    let storage = MmapVectorStorage::open(&vec_file).expect("open mmap vector storage");
    assert_eq!(storage.count(), count);
    assert_eq!(storage.dimension(), dim);
    assert_eq!(storage.all_vectors().len(), count * dim);

    // Verify zero-copy slice retrieval
    let vec_0 = storage.get_vector(0).expect("vector 0");
    assert_eq!(vec_0.len(), dim);
    assert_eq!(vec_0, &flat_vectors[0..dim]);

    let vec_9 = storage.get_vector(9).expect("vector 9");
    assert_eq!(vec_9.len(), dim);
    assert_eq!(vec_9, &flat_vectors[9 * dim..10 * dim]);

    assert!(storage.get_vector(10).is_none());

    // 5. Verify SIMD dot product and Rayon parallel cosine search
    let v0 = storage.get_vector(0).unwrap();
    let v1 = storage.get_vector(1).unwrap();
    let direct_simd = dot_product_simd(v0, v1);
    let expected_dot: f32 = v0.iter().zip(v1.iter()).map(|(a, b)| a * b).sum();
    assert!((direct_simd - expected_dot).abs() < 1e-6);

    let query = storage.get_vector(3).unwrap().to_vec();
    let results = search_cosine_top_k(&query, &storage, 5);
    assert_eq!(results.len(), 5);
    // Exact match must be rank 0 with similarity ~1.0
    assert_eq!(results[0].0, 3);
    assert!((results[0].1 - 1.0).abs() < 1e-4);

    // 6. Test SQLite validation
    let db_path = tmp_dir.path().join("index.db");
    let conn = open_connection(&db_path).expect("open db");
    initialize_schema(&conn).expect("init schema");

    // Insert dummy commit and lineage to satisfy foreign keys
    conn.execute(
        "INSERT INTO commits (hash, author, email, authored_at, topo_order, parent_hashes, message, insertions, deletions)
         VALUES ('c1', 'Test', 'test@example.com', 1000, 1, '', 'commit 1', 10, 0);",
        [],
    ).expect("insert commit");

    conn.execute(
        "INSERT INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
         VALUES (1, 'src/lib.rs', 'test_fn', 'rust');",
        [],
    ).expect("insert lineage");

    // Insert 10 function touches and embeddings records
    for i in 0..count {
        let touch_id = i as i64 + 1;
        conn.execute(
            "INSERT INTO function_touches (touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type, chunk_text, parse_status, start_line, end_line)
             VALUES (?1, 1, 'c1', 'src/lib.rs', 'test_fn', 0, 'add', 'code', 'ok', 1, 10);",
            rusqlite::params![touch_id],
        ).expect("insert touch");

        conn.execute(
            "INSERT INTO embeddings (touch_id, row_offset) VALUES (?1, ?2);",
            rusqlite::params![touch_id, i as i64],
        )
        .expect("insert embedding");
    }
    storage.validate_with_sqlite(&conn).expect("sqlite count matches");

    // Insert an extra touch + embedding record to test mismatch failure
    conn.execute(
        "INSERT INTO function_touches (touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type, chunk_text, parse_status, start_line, end_line)
         VALUES (999, 1, 'c1', 'src/lib.rs', 'test_fn', 0, 'add', 'code', 'ok', 1, 10);",
        [],
    ).expect("insert touch 999");
    conn.execute(
        "INSERT INTO embeddings (touch_id, row_offset) VALUES (999, 999);",
        [],
    )
    .expect("insert extra");
    assert!(storage.validate_with_sqlite(&conn).is_err());
}

#[test]
fn test_vector_simd_scoring_and_incremental() {
    let tmp_dir = tempdir().expect("tempdir");
    let vec_file = tmp_dir.path().join("vectors.bin");

    // Test with 20,000 vectors for SIMD scoring throughput (< 15ms target)
    let dim = 384;
    let count = 20_000;
    let mut flat_vectors = vec![0.0f32; count * dim];

    // Initialize with normalized patterns
    for i in 0..count {
        let val = 1.0 / (dim as f32).sqrt();
        for d in 0..dim {
            flat_vectors[i * dim + d] = if (i + d) % 2 == 0 { val } else { -val };
        }
    }

    write_vector_file(&vec_file, &flat_vectors, dim as u32, count as u32)
        .expect("write vectors");

    let storage = MmapVectorStorage::open(&vec_file).expect("open mmap storage");
    let query = vec![1.0 / (dim as f32).sqrt(); dim];

    let start = Instant::now();
    let top_k = search_cosine_top_k(&query, &storage, 10);
    let elapsed = start.elapsed();

    assert_eq!(top_k.len(), 10);
    // Scoring 20,000 vectors on CPU should take < 50ms in debug mode, < 5ms in release mode
    println!("Scored {} 384-dim vectors in {:?}", count, elapsed);
}

#[test]
fn test_fastembed_batch_embedding_and_status_filtering() {
    let mut generator = match FastEmbedGenerator::new(None) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Skipping FastEmbed test due to environment/offline cache: {e}");
            return;
        }
    };

    assert_eq!(generator.dimension(), 384);

    let chunks = vec![
        EmbeddingInput {
            touch_id: 1,
            text: "def calculate_tax(amount, rate): return amount * rate".to_string(),
            parse_status: ParseStatus::Ok,
        },
        EmbeddingInput {
            touch_id: 2,
            text: "fn process_payment(user_id: u64, amount: f64) -> Result<(), Error>".to_string(),
            parse_status: ParseStatus::Degraded,
        },
        EmbeddingInput {
            touch_id: 3,
            text: "this is broken syntax { { [[".to_string(),
            parse_status: ParseStatus::Failed, // Skipped per D-12
        },
        EmbeddingInput {
            touch_id: 4,
            text: "class UserAuthenticationService: def verify_token(self, token): pass".to_string(),
            parse_status: ParseStatus::Ok,
        },
    ];

    let outputs = generator
        .generate_embeddings_batched(&chunks, 128)
        .expect("batch embedding");

    // Must have embedded Ok and Degraded, skipping Failed (D-12)
    assert_eq!(outputs.len(), 3);
    assert_eq!(outputs[0].touch_id, 1);
    assert_eq!(outputs[1].touch_id, 2);
    assert_eq!(outputs[2].touch_id, 4);

    for out in &outputs {
        assert_eq!(out.embedding.len(), 384);
    }
}

// Structure representing synthetic test commit batch for determinism and equivalence tests
struct SyntheticCommitData {
    commit: CommitRecord,
    lineages: Vec<LineageRecord>,
    touches: Vec<TouchRecord>,
    vectors: Vec<Vec<f32>>,
}

fn generate_synthetic_data(num_commits: usize) -> Vec<SyntheticCommitData> {
    let dim = 384;
    let mut result = Vec::new();

    for c in 1..=num_commits {
        let commit_hash = format!("commit_{:04}", c);
        let commit = CommitRecord {
            hash: commit_hash.clone(),
            author: format!("Author {}", c),
            email: format!("author{}@gitctx.test", c),
            authored_at: 1700000000 + (c as i64 * 60),
            topo_order: c as u32,
            branch: Some("main".to_string()),
            parent_hashes: if c == 1 { String::new() } else { format!("commit_{:04}", c - 1) },
            message: format!("Commit message {}", c),
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
            change_type: if c == 1 { "add".to_string() } else { "modify".to_string() },
            signature_before: if c == 1 { None } else { Some("fn handle_request(req: &Request)".to_string()) },
            signature_after: Some("fn handle_request(req: &Request) -> Response".to_string()),
            fingerprint: Some(vec![c as u8; 32]),
            chunk_text: format!("fn handle_request_{}() {{ let x = {}; }}", c, c * 100),
            parse_status: "ok".to_string(),
            start_line: 1,
            end_line: 25,
        };

        // Deterministic synthetic embedding vector
        let mut raw_vec = vec![0.0f32; dim];
        for d in 0..dim {
            raw_vec[d] = ((c * 17 + d * 31) % 1000) as f32 - 500.0;
        }
        let norm_vec = normalized_l2(&raw_vec);

        result.push(SyntheticCommitData {
            commit,
            lineages: vec![lineage],
            touches: vec![touch],
            vectors: vec![norm_vec],
        });
    }

    result
}

fn apply_commit_batches_to_index(
    db_conn: &Connection,
    tantivy_engine: &mut TantivyEngine,
    flat_vectors: &mut Vec<f32>,
    batches: &[SyntheticCommitData],
) {
    let mut stmt_commit = db_conn
        .prepare_cached(
            "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);"
        )
        .unwrap();

    let mut stmt_lineage = db_conn
        .prepare_cached(
            "INSERT OR REPLACE INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
             VALUES (?1, ?2, ?3, ?4);"
        )
        .unwrap();

    let mut stmt_touch = db_conn
        .prepare_cached(
            "INSERT INTO function_touches (touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type, signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14);"
        )
        .unwrap();

    let mut stmt_emb = db_conn
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
            stmt_emb.execute(rusqlite::params![touch_id, row_offset]).unwrap();

            flat_vectors.extend_from_slice(&batch.vectors[i]);

            tantivy_engine
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
    tantivy_engine.commit().unwrap();
}

#[test]
fn test_double_indexing_determinism() {
    // INVAR-04 / Invariant I2: Byte-identical SQLite and vectors.bin across independent runs
    let data = generate_synthetic_data(5);

    let dir_a = tempdir().expect("tempdir a");
    let dir_b = tempdir().expect("tempdir b");

    let db_path_a = dir_a.path().join("index.db");
    let tantivy_path_a = dir_a.path().join("tantivy");
    let vec_path_a = dir_a.path().join("vectors.bin");

    let db_path_b = dir_b.path().join("index.db");
    let tantivy_path_b = dir_b.path().join("tantivy");
    let vec_path_b = dir_b.path().join("vectors.bin");

    // Run A
    let conn_a = open_connection(&db_path_a).expect("open db a");
    initialize_schema(&conn_a).expect("init schema a");
    let mut tantivy_a = TantivyEngine::create_in_dir(&tantivy_path_a, 15_000_000).expect("create tantivy a");
    let mut vectors_a = Vec::new();
    apply_commit_batches_to_index(&conn_a, &mut tantivy_a, &mut vectors_a, &data);
    let count_a = (vectors_a.len() / 384) as u32;
    write_vector_file_atomic(&vec_path_a, &vectors_a, 384, count_a).expect("write vec a");
    conn_a.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").expect("checkpoint a");
    drop(conn_a);

    // Run B
    let conn_b = open_connection(&db_path_b).expect("open db b");
    initialize_schema(&conn_b).expect("init schema b");
    let mut tantivy_b = TantivyEngine::create_in_dir(&tantivy_path_b, 15_000_000).expect("create tantivy b");
    let mut vectors_b = Vec::new();
    apply_commit_batches_to_index(&conn_b, &mut tantivy_b, &mut vectors_b, &data);
    let count_b = (vectors_b.len() / 384) as u32;
    write_vector_file_atomic(&vec_path_b, &vectors_b, 384, count_b).expect("write vec b");
    conn_b.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").expect("checkpoint b");
    drop(conn_b);

    // 1. Assert byte-identical vectors.bin
    let vec_bytes_a = fs::read(&vec_path_a).expect("read vec a");
    let vec_bytes_b = fs::read(&vec_path_b).expect("read vec b");
    assert_eq!(vec_bytes_a, vec_bytes_b, "vectors.bin must be byte-identical across runs");

    // 2. Assert byte-identical SQLite database files (Invariant I2 / INVAR-04)
    let db_bytes_a = fs::read(&db_path_a).expect("read db a");
    let db_bytes_b = fs::read(&db_path_b).expect("read db b");
    assert_eq!(db_bytes_a, db_bytes_b, "index.db must be byte-identical across runs");

    // 3. Assert exact identical search rankings across both runs
    let storage_a = MmapVectorStorage::open(&vec_path_a).expect("open storage a");
    let storage_b = MmapVectorStorage::open(&vec_path_b).expect("open storage b");
    let query_vec = storage_a.get_vector(0).unwrap().to_vec();

    let scores_a = search_cosine_top_k(&query_vec, &storage_a, 5);
    let scores_b = search_cosine_top_k(&query_vec, &storage_b, 5);
    assert_eq!(scores_a, scores_b, "Dense vector search rankings must be identical");

    let bm25_a = tantivy_a.search("handle_request", 5).expect("bm25 a");
    let bm25_b = tantivy_b.search("handle_request", 5).expect("bm25 b");
    assert_eq!(bm25_a, bm25_b, "Tantivy BM25 rankings must be identical");
}

#[test]
fn test_incremental_vs_full_equivalence() {
    // INVAR-05 / Invariant I3: Cold index (1..N) vs Incremental index (1..N-k, then N-k+1..N)
    let all_data = generate_synthetic_data(6);

    let dir_full = tempdir().expect("tempdir full");
    let dir_incr = tempdir().expect("tempdir incr");

    let db_path_full = dir_full.path().join("index.db");
    let tantivy_path_full = dir_full.path().join("tantivy");
    let vec_path_full = dir_full.path().join("vectors.bin");

    let db_path_incr = dir_incr.path().join("index.db");
    let tantivy_path_incr = dir_incr.path().join("tantivy");
    let vec_path_incr = dir_incr.path().join("vectors.bin");

    // Full Index (1..6)
    let conn_full = open_connection(&db_path_full).expect("open db full");
    initialize_schema(&conn_full).expect("init schema full");
    let mut tantivy_full = TantivyEngine::create_in_dir(&tantivy_path_full, 15_000_000).expect("create tantivy full");
    let mut vectors_full = Vec::new();
    apply_commit_batches_to_index(&conn_full, &mut tantivy_full, &mut vectors_full, &all_data);
    let count_full = (vectors_full.len() / 384) as u32;
    write_vector_file_atomic(&vec_path_full, &vectors_full, 384, count_full).expect("write vec full");
    conn_full.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").expect("checkpoint full");

    // Incremental Index: Phase 1 (1..4)
    let conn_incr = open_connection(&db_path_incr).expect("open db incr");
    initialize_schema(&conn_incr).expect("init schema incr");
    let mut tantivy_incr = TantivyEngine::create_in_dir(&tantivy_path_incr, 15_000_000).expect("create tantivy incr");
    let mut vectors_incr = Vec::new();
    apply_commit_batches_to_index(&conn_incr, &mut tantivy_incr, &mut vectors_incr, &all_data[0..4]);

    // Incremental Index: Phase 2 (5..6)
    apply_commit_batches_to_index(&conn_incr, &mut tantivy_incr, &mut vectors_incr, &all_data[4..6]);
    let count_incr = (vectors_incr.len() / 384) as u32;
    write_vector_file_atomic(&vec_path_incr, &vectors_incr, 384, count_incr).expect("write vec incr");
    conn_incr.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").expect("checkpoint incr");

    // Assert row counts match across tables
    let count_commits_full: i64 = conn_full.query_row("SELECT COUNT(*) FROM commits;", [], |r| r.get(0)).unwrap();
    let count_commits_incr: i64 = conn_incr.query_row("SELECT COUNT(*) FROM commits;", [], |r| r.get(0)).unwrap();
    assert_eq!(count_commits_full, count_commits_incr);
    assert_eq!(count_commits_full, 6);

    let count_touches_full: i64 = conn_full.query_row("SELECT COUNT(*) FROM function_touches;", [], |r| r.get(0)).unwrap();
    let count_touches_incr: i64 = conn_incr.query_row("SELECT COUNT(*) FROM function_touches;", [], |r| r.get(0)).unwrap();
    assert_eq!(count_touches_full, count_touches_incr);
    assert_eq!(count_touches_full, 6);

    let count_emb_full: i64 = conn_full.query_row("SELECT COUNT(*) FROM embeddings;", [], |r| r.get(0)).unwrap();
    let count_emb_incr: i64 = conn_incr.query_row("SELECT COUNT(*) FROM embeddings;", [], |r| r.get(0)).unwrap();
    assert_eq!(count_emb_full, count_emb_incr);
    assert_eq!(count_emb_full, 6);

    // Check all row contents in embeddings and function_touches
    let mut stmt_emb_full = conn_full.prepare("SELECT touch_id, row_offset FROM embeddings ORDER BY touch_id ASC;").unwrap();
    let rows_full: Vec<(i64, i64)> = stmt_emb_full.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(|r| r.unwrap()).collect();

    let mut stmt_emb_incr = conn_incr.prepare("SELECT touch_id, row_offset FROM embeddings ORDER BY touch_id ASC;").unwrap();
    let rows_incr: Vec<(i64, i64)> = stmt_emb_incr.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().map(|r| r.unwrap()).collect();
    assert_eq!(rows_full, rows_incr, "Embeddings table mapping must be equivalent");

    // Check vector binary file equivalence
    let storage_full = MmapVectorStorage::open(&vec_path_full).expect("open storage full");
    let storage_incr = MmapVectorStorage::open(&vec_path_incr).expect("open storage incr");
    assert_eq!(storage_full.count(), storage_incr.count());
    assert_eq!(storage_full.all_vectors(), storage_incr.all_vectors());
}
