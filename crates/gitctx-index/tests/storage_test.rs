use gitctx_core::config::IndexConfig;
use gitctx_core::error::CoreError;
use gitctx_index::storage::{
    open_connection, AtomicStorageManager, BackgroundWriter, CommitRecord, LineageRecord,
    StorageEngine, TouchRecord, CURRENT_SCHEMA_VERSION,
};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_sqlite_storage_and_atomic_swap() {
    let tmp_dir = tempdir().expect("Failed to create tempdir");
    let mut manager = AtomicStorageManager::new(tmp_dir.path());

    // 1. Open temporary database and apply WAL PRAGMAs
    let conn = open_connection(manager.tmp_db_path()).expect("Failed to open tmp db");

    // 2. Start background writer with batching
    let writer = BackgroundWriter::new(conn, 10);

    // 3. Push commit records with topological ordering
    let commits = vec![
        CommitRecord {
            hash: "c001".to_string(),
            author: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            authored_at: 1000,
            topo_order: 1,
            branch: Some("main".to_string()),
            parent_hashes: String::new(),
            message: "Initial commit".to_string(),
            insertions: 50,
            deletions: 0,
        },
        CommitRecord {
            hash: "c002".to_string(),
            author: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            authored_at: 2000,
            topo_order: 2,
            branch: Some("main".to_string()),
            parent_hashes: "c001".to_string(),
            message: "Refactor auth".to_string(),
            insertions: 20,
            deletions: 10,
        },
        CommitRecord {
            hash: "c003".to_string(),
            author: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
            authored_at: 3000,
            topo_order: 3,
            branch: Some("main".to_string()),
            parent_hashes: "c002".to_string(),
            message: "Optimize login".to_string(),
            insertions: 5,
            deletions: 2,
        },
    ];
    writer
        .push_commits(commits)
        .expect("Failed to push commits");

    // 4. Push lineage
    let lineages = vec![LineageRecord {
        lineage_id: 42,
        current_path: "src/auth.rs".to_string(),
        current_qualified_name: "auth::login".to_string(),
        language: "rust".to_string(),
    }];
    writer
        .push_lineages(lineages)
        .expect("Failed to push lineages");

    // 5. Push touches (out of topological order over the channel to test ordering on retrieval)
    let touches = vec![
        TouchRecord {
            touch_id: Some(103),
            lineage_id: 42,
            commit_hash: "c003".to_string(),
            path: "src/auth.rs".to_string(),
            qualified_name: "auth::login".to_string(),
            arity: 2,
            change_type: "modify".to_string(),
            signature_before: Some("fn login(u: &str, p: &str)".to_string()),
            signature_after: Some("fn login(u: &str, p: &str) -> bool".to_string()),
            fingerprint: Some(vec![1, 2, 3, 4]),
            chunk_text: "fn login optimized".to_string(),
            parse_status: "ok".to_string(),
            start_line: 10,
            end_line: 25,
        },
        TouchRecord {
            touch_id: Some(101),
            lineage_id: 42,
            commit_hash: "c001".to_string(),
            path: "src/auth.rs".to_string(),
            qualified_name: "auth::login".to_string(),
            arity: 2,
            change_type: "add".to_string(),
            signature_before: None,
            signature_after: Some("fn login(u: &str, p: &str)".to_string()),
            fingerprint: Some(vec![1, 2, 3, 4]),
            chunk_text: "fn login initial".to_string(),
            parse_status: "ok".to_string(),
            start_line: 1,
            end_line: 15,
        },
        TouchRecord {
            touch_id: Some(102),
            lineage_id: 42,
            commit_hash: "c002".to_string(),
            path: "src/auth.rs".to_string(),
            qualified_name: "auth::login".to_string(),
            arity: 2,
            change_type: "modify".to_string(),
            signature_before: Some("fn login(u: &str, p: &str)".to_string()),
            signature_after: Some("fn login(u: &str, p: &str)".to_string()),
            fingerprint: Some(vec![1, 2, 3, 4]),
            chunk_text: "fn login refactored".to_string(),
            parse_status: "ok".to_string(),
            start_line: 5,
            end_line: 20,
        },
    ];
    writer
        .push_touches(touches)
        .expect("Failed to push touches");

    // 6. Record state
    let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "bge-small-en-v1.5", "code", 500, 1);
    writer
        .set_state("schema_version", config.schema_version.to_string())
        .expect("Failed to set schema_version");
    writer
        .set_state("config_hash", config.compute_config_hash().as_str())
        .expect("Failed to set config_hash");

    // 7. Finish writer and checkpoint WAL
    let finished_conn = writer.finish().expect("Failed to finish background writer");
    AtomicStorageManager::checkpoint_and_close(finished_conn)
        .expect("Failed to checkpoint and close");

    // 8. Commit atomic swap
    assert!(manager.tmp_db_path().exists());
    assert!(!manager.db_path().exists());
    manager.commit_swap().expect("Failed to commit atomic swap");
    assert!(!manager.tmp_db_path().exists());
    assert!(manager.db_path().exists());

    // 9. Open via StorageEngine and read back touches by lineage
    let engine = StorageEngine::open(manager.db_path(), &config)
        .expect("Failed to open StorageEngine with valid config");

    let retrieved = engine
        .query_touches_by_lineage(42)
        .expect("Failed to query touches by lineage");

    assert_eq!(retrieved.len(), 3);
    // Commits were inserted c003, c001, c002.
    // Query MUST return ordered by c.topo_order ASC -> c001 (topo=1), c002 (topo=2), c003 (topo=3).
    assert_eq!(retrieved[0].commit_hash, "c001");
    assert_eq!(retrieved[0].touch_id, Some(101));
    assert_eq!(retrieved[0].change_type, "add");

    assert_eq!(retrieved[1].commit_hash, "c002");
    assert_eq!(retrieved[1].touch_id, Some(102));
    assert_eq!(retrieved[1].change_type, "modify");

    assert_eq!(retrieved[2].commit_hash, "c003");
    assert_eq!(retrieved[2].touch_id, Some(103));
    assert_eq!(retrieved[2].change_type, "modify");
}

#[test]
fn test_config_hash_mismatch_refusal() {
    let tmp_dir = tempdir().expect("Failed to create tempdir");
    let mut manager = AtomicStorageManager::new(tmp_dir.path());

    let original_config =
        IndexConfig::new(CURRENT_SCHEMA_VERSION, "bge-small-en-v1.5", "code", 500, 1);

    // Create database with original_config
    {
        let conn = open_connection(manager.tmp_db_path()).expect("Failed to open tmp db");
        let writer = BackgroundWriter::new(conn, 10);
        writer
            .set_state("schema_version", original_config.schema_version.to_string())
            .expect("Failed to set schema_version");
        writer
            .set_state(
                "config_hash",
                original_config.compute_config_hash().as_str(),
            )
            .expect("Failed to set config_hash");

        let finished_conn = writer.finish().expect("Failed to finish writer");
        AtomicStorageManager::checkpoint_and_close(finished_conn)
            .expect("Failed to checkpoint and close");
        manager.commit_swap().expect("Failed to commit swap");
    }

    // 1. Opening with identical config must succeed
    assert!(StorageEngine::open(manager.db_path(), &original_config).is_ok());

    // 2. Opening with modified model_id must fail fast with diagnostic
    let altered_model_config =
        IndexConfig::new(CURRENT_SCHEMA_VERSION, "all-MiniLM-L6-v2", "code", 500, 1);
    match StorageEngine::open(manager.db_path(), &altered_model_config) {
        Err(CoreError::ConfigMismatch { actual, .. }) => {
            assert!(
                actual.contains("Index schema version mismatch. Run 'gitctx index' to rebuild."),
                "Error message missing rebuild diagnostic: {actual}"
            );
        }
        other => panic!("Expected ConfigMismatch error, got: {other:?}"),
    }

    // 3. Opening with modified schema_version must fail fast with diagnostic
    let altered_schema_config = IndexConfig::new(
        CURRENT_SCHEMA_VERSION + 1,
        "bge-small-en-v1.5",
        "code",
        500,
        1,
    );
    match StorageEngine::open(manager.db_path(), &altered_schema_config) {
        Err(CoreError::ConfigMismatch { actual, .. }) => {
            assert!(
                actual.contains("Index schema version mismatch. Run 'gitctx index' to rebuild."),
                "Error message missing rebuild diagnostic: {actual}"
            );
        }
        other => panic!("Expected ConfigMismatch error, got: {other:?}"),
    }
}

#[test]
fn test_atomic_replacement_and_crash_cleanup() {
    let tmp_dir = tempdir().expect("Failed to create tempdir");

    // Case 1: Aborted / crashed run cleans up .tmp files without affecting destination
    {
        let manager = AtomicStorageManager::new(tmp_dir.path());
        fs::write(manager.tmp_db_path(), b"partial sqlite data").unwrap();
        fs::write(manager.tmp_vectors_path(), b"partial vectors").unwrap();
        fs::create_dir_all(manager.tmp_tantivy_dir()).unwrap();
        fs::write(manager.tmp_tantivy_dir().join("meta.json"), b"{}").unwrap();

        assert!(manager.tmp_db_path().exists());
        assert!(manager.tmp_vectors_path().exists());
        assert!(manager.tmp_tantivy_dir().exists());

        // Manager drops without commit_swap -> drop cleanup triggers
        drop(manager);
    }

    // Verify temp files removed
    let manager = AtomicStorageManager::new(tmp_dir.path());
    assert!(!manager.tmp_db_path().exists());
    assert!(!manager.tmp_vectors_path().exists());
    assert!(!manager.tmp_tantivy_dir().exists());
    assert!(!manager.db_path().exists());

    // Case 2: Successful swap replaces existing active files
    {
        // Create initial active files
        fs::write(manager.db_path(), b"active v1").unwrap();
        fs::write(manager.vectors_path(), b"vectors v1").unwrap();
        fs::create_dir_all(manager.tantivy_dir()).unwrap();
        fs::write(manager.tantivy_dir().join("meta.json"), b"v1").unwrap();

        let mut swap_manager = AtomicStorageManager::new(tmp_dir.path());
        fs::write(swap_manager.tmp_db_path(), b"active v2").unwrap();
        fs::write(swap_manager.tmp_vectors_path(), b"vectors v2").unwrap();
        fs::create_dir_all(swap_manager.tmp_tantivy_dir()).unwrap();
        fs::write(swap_manager.tmp_tantivy_dir().join("meta.json"), b"v2").unwrap();

        swap_manager.commit_swap().expect("Failed to commit swap");

        assert_eq!(fs::read(swap_manager.db_path()).unwrap(), b"active v2");
        assert_eq!(
            fs::read(swap_manager.vectors_path()).unwrap(),
            b"vectors v2"
        );
        assert_eq!(
            fs::read(swap_manager.tantivy_dir().join("meta.json")).unwrap(),
            b"v2"
        );
        assert!(!swap_manager.tmp_db_path().exists());
        assert!(!swap_manager.tmp_vectors_path().exists());
        assert!(!swap_manager.tmp_tantivy_dir().exists());
    }
}

#[test]
fn test_explain_cache_isolation() {
    let tmp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = tmp_dir.path().join("index.db");

    let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "bge-small-en-v1.5", "code", 500, 1);
    let mut engine = StorageEngine::open_raw(&db_path).expect("Failed to open raw storage");
    engine
        .record_state(&config)
        .expect("Failed to record state");

    // 1. Insert code commit, lineage, and function touch
    let conn = engine.connection_mut();
    conn.execute(
        "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
         VALUES ('c100', 'Dev', 'dev@example.com', 1000, 1, 'main', '', 'feat: add calculate_total', 10, 0);",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
         VALUES (99, 'src/math.rs', 'math::calculate_total', 'rust');",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO function_touches (
            touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type,
            signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line
         ) VALUES (501, 99, 'c100', 'src/math.rs', 'math::calculate_total', 1, 'add',
            NULL, 'fn calculate_total(items: &[Item]) -> u64', NULL, 'fn calculate_total(items: &[Item]) -> u64 { items.iter().map(|i| i.price).sum() }', 'ok', 1, 10);",
        [],
    ).unwrap();

    // 2. Put LLM explanation into isolated explain_cache
    let explain_cache = engine.explain_cache();
    let cache_key = "hash_c100_math_calculate_total_v1";
    let llm_model = "gpt-4o-mini";
    let prompt_ver = "v1.2";
    let llm_text =
        "This function sums item prices using iterator map. (LLM_SYNTHETIC_EXPLANATION_TOKEN_9988)";
    let timestamp = 1700000000;

    explain_cache
        .put(cache_key, llm_model, prompt_ver, llm_text, timestamp)
        .expect("Failed to put into explain_cache");

    // 3. Verify explain_cache lookup
    let cached = explain_cache
        .get(cache_key)
        .expect("Failed to get from explain_cache")
        .expect("Expected cached explanation entry");

    assert_eq!(cached.cache_key, cache_key);
    assert_eq!(cached.model, llm_model);
    assert_eq!(cached.prompt_version, prompt_ver);
    assert_eq!(cached.text, llm_text);
    assert_eq!(cached.created_at, timestamp);
    assert_eq!(explain_cache.count().unwrap(), 1);

    // 4. Verify non-existent key returns None
    let missing = explain_cache.get("non_existent_key").unwrap();
    assert!(missing.is_none());

    // 5. Enforce Invariant I5: Verify touch extraction queries NEVER leak explain_cache text
    let touches_by_lineage = engine.query_touches_by_lineage(99).unwrap();
    assert_eq!(touches_by_lineage.len(), 1);
    assert_eq!(
        touches_by_lineage[0].qualified_name,
        "math::calculate_total"
    );
    assert_eq!(
        touches_by_lineage[0].chunk_text,
        "fn calculate_total(items: &[Item]) -> u64 { items.iter().map(|i| i.price).sum() }"
    );
    assert!(
        !touches_by_lineage[0]
            .chunk_text
            .contains("LLM_SYNTHETIC_EXPLANATION_TOKEN"),
        "Invariant I5 violation: LLM explanation leaked into touch chunk_text"
    );

    let touches_by_qualname = engine
        .query_touches_by_qualname("math::calculate_total")
        .unwrap();
    assert_eq!(touches_by_qualname.len(), 1);
    assert!(
        !touches_by_qualname[0]
            .chunk_text
            .contains("LLM_SYNTHETIC_EXPLANATION_TOKEN"),
        "Invariant I5 violation: LLM explanation leaked into touch chunk_text"
    );
}

#[test]
fn test_schema_v2_cochange_tables_and_queries() {
    let tmp_dir = tempdir().expect("Failed to create tempdir");
    let db_path = tmp_dir.path().join("index.db");
    let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "BGESmallENV15", "code", 500, 1);

    assert_eq!(CURRENT_SCHEMA_VERSION, 2);

    let conn = open_connection(&db_path).expect("Failed to open connection");
    gitctx_index::storage::initialize_schema(&conn).expect("Failed to initialize schema");
    conn.execute(
        "INSERT INTO index_state (key, value) VALUES ('schema_version', ?1), ('config_hash', ?2);",
        rusqlite::params![
            CURRENT_SCHEMA_VERSION.to_string(),
            config.compute_config_hash().as_str()
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO cochange_file_metrics (entity_a, entity_b, support, weighted_support, confidence_a_to_b, confidence_b_to_a, lift)
         VALUES ('src/a.rs', 'src/b.rs', 5, 4.8, 0.83, 0.71, 2.5);",
        [],
    ).unwrap();

    conn.execute(
        "INSERT INTO cochange_function_metrics (entity_a, entity_b, support, weighted_support, confidence_a_to_b, confidence_b_to_a, lift)
         VALUES ('mod::foo', 'mod::bar', 3, 2.9, 0.75, 0.60, 3.1);",
        [],
    ).unwrap();

    let engine = StorageEngine::open(&db_path, &config).expect("Failed to open engine");

    // Query file metrics for entity_a ('src/a.rs')
    let res_a = engine
        .query_cochange_file_metrics_for("src/a.rs", 10)
        .unwrap();
    assert_eq!(res_a.len(), 1);
    assert_eq!(res_a[0].entity_b, "src/b.rs");
    assert_eq!(res_a[0].support, 5);

    // Query file metrics for entity_b ('src/b.rs')
    let res_b = engine
        .query_cochange_file_metrics_for("src/b.rs", 10)
        .unwrap();
    assert_eq!(res_b.len(), 1);
    assert_eq!(res_b[0].entity_a, "src/a.rs");

    // Query function metrics
    let res_func = engine
        .query_cochange_function_metrics_for("mod::foo", 10)
        .unwrap();
    assert_eq!(res_func.len(), 1);
    assert_eq!(res_func[0].entity_b, "mod::bar");

    // Query global metrics
    let global_files = engine.query_global_file_cochange_metrics(10).unwrap();
    assert_eq!(global_files.len(), 1);
    let global_funcs = engine.query_global_function_cochange_metrics(10).unwrap();
    assert_eq!(global_funcs.len(), 1);
}

#[test]
fn test_staged_atomic_rotation() {
    let tmp_dir = tempdir().expect("Failed to create tempdir");
    let mut manager = AtomicStorageManager::new(tmp_dir.path());

    // Write initial existing files
    fs::write(manager.db_path(), b"INITIAL_DB").unwrap();
    fs::write(manager.vectors_path(), b"INITIAL_VEC").unwrap();
    fs::create_dir_all(manager.tantivy_dir()).unwrap();
    fs::write(manager.tantivy_dir().join("meta.json"), b"INITIAL_TANTIVY").unwrap();

    // Write new temp files
    fs::write(manager.tmp_db_path(), b"NEW_DB").unwrap();
    fs::write(manager.tmp_vectors_path(), b"NEW_VEC").unwrap();
    fs::create_dir_all(manager.tmp_tantivy_dir()).unwrap();
    fs::write(manager.tmp_tantivy_dir().join("meta.json"), b"NEW_TANTIVY").unwrap();

    // Commit swap
    manager.commit_swap().expect("commit_swap should succeed");

    assert_eq!(fs::read(manager.db_path()).unwrap(), b"NEW_DB");
    assert_eq!(fs::read(manager.vectors_path()).unwrap(), b"NEW_VEC");
    assert_eq!(
        fs::read(manager.tantivy_dir().join("meta.json")).unwrap(),
        b"NEW_TANTIVY"
    );

    // Verify .bak files were cleaned up
    assert!(!manager.db_path().with_extension("bak").exists());
    assert!(!manager.vectors_path().with_extension("bak").exists());
    assert!(!manager.tantivy_dir().with_extension("bak").exists());
}

#[test]
fn test_staged_atomic_rotation_cleans_stale_wal() {
    let tmp_dir = tempdir().expect("Failed to create tempdir");
    let mut manager = AtomicStorageManager::new(tmp_dir.path());

    // Write initial existing files and a stale -wal / -shm
    fs::write(manager.db_path(), b"INITIAL_DB").unwrap();
    let stale_wal = PathBuf::from(format!("{}-wal", manager.db_path().display()));
    let stale_shm = PathBuf::from(format!("{}-shm", manager.db_path().display()));
    fs::write(&stale_wal, b"STALE_WAL_DATA").unwrap();
    fs::write(&stale_shm, b"STALE_SHM_DATA").unwrap();
    assert!(stale_wal.exists());
    assert!(stale_shm.exists());

    // Write new temp db
    fs::write(manager.tmp_db_path(), b"NEW_DB").unwrap();

    // Commit swap
    manager.commit_swap().expect("commit_swap should succeed");

    assert_eq!(fs::read(manager.db_path()).unwrap(), b"NEW_DB");
    // Stale WAL and SHM files must be removed so SQLite doesn't attempt replay
    assert!(!stale_wal.exists());
    assert!(!stale_shm.exists());
}
