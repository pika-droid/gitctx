use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{
    open_connection, AtomicStorageManager, BackgroundWriter, CommitRecord, LineageRecord,
    StorageEngine, TouchRecord, CURRENT_SCHEMA_VERSION,
};
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
            parent_hashes: "".to_string(),
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
    writer.push_commits(commits).expect("Failed to push commits");

    // 4. Push lineage
    let lineages = vec![LineageRecord {
        lineage_id: 42,
        current_path: "src/auth.rs".to_string(),
        current_qualified_name: "auth::login".to_string(),
        language: "rust".to_string(),
    }];
    writer.push_lineages(lineages).expect("Failed to push lineages");

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
    writer.push_touches(touches).expect("Failed to push touches");

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
