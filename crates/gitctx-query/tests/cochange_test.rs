//! Integration tests for the co-change extraction, metrics, graph, and impact pipeline.
//!
//! Uses a fresh in-memory SQLite database with fixture commits and function touches
//! to verify the full Phase 7 pipeline end-to-end.

use gitctx_query::cochange::config::CoChangeConfig;
use gitctx_query::cochange::extractor::CoChangeExtractor;
use gitctx_query::cochange::graph::{CoChangeGraph, EntityType};
use gitctx_query::cochange::impact::ImpactAnalyzer;
use gitctx_query::cochange::metrics::MetricsComputer;
use gitctx_index::storage::StorageEngine;

/// Helper: create a fresh StorageEngine with schema initialized.
fn fresh_engine(dir: &tempfile::TempDir) -> StorageEngine {
    let db_path = dir.path().join("test.db");
    StorageEngine::open_raw(&db_path).expect("Failed to open storage")
}

/// Helper: insert commits and touches into the storage engine.
fn seed_fixture(engine: &StorageEngine) {
    let conn = engine.connection();

    // Insert 5 commits with varying file counts and authors
    let commits = vec![
        // Normal commit touching 2 files
        ("aaa111", "alice@dev.com", 1_700_000_000_i64, 1_u32, "feat: add parser", 10_u32, 5_u32),
        // Normal commit touching 2 files (same as commit 1 pair)
        ("bbb222", "bob@dev.com", 1_700_100_000, 2, "refactor: clean up parser", 15, 3),
        // Normal commit touching 3 files
        ("ccc333", "carol@dev.com", 1_700_200_000, 3, "feat: add formatter", 20, 8),
        // Bot commit — should be filtered
        ("ddd444", "dependabot[bot]@users.noreply.github.com", 1_700_300_000, 4, "chore: bump deps", 5, 2),
        // Fix commit touching a coupled entity within 7 days of commit 3
        ("eee555", "alice@dev.com", 1_700_250_000, 5, "fix: null pointer in formatter", 3, 1),
    ];

    for (hash, email, authored_at, topo, message, ins, del) in &commits {
        conn.execute(
            "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, '', ?6, ?7, ?8)",
            rusqlite::params![hash, email.split('@').next().unwrap_or("unknown"), email, authored_at, topo, message, ins, del],
        ).expect("insert commit");
    }

    // Insert lineages
    let lineages = vec![
        (1_u64, "src/parser.rs", "parser::parse", "rust"),
        (2, "src/lexer.rs", "lexer::tokenize", "rust"),
        (3, "src/formatter.rs", "formatter::format", "rust"),
        (4, "src/utils.rs", "utils::helper", "rust"),
    ];

    for (lid, path, qn, lang) in &lineages {
        conn.execute(
            "INSERT INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![lid, path, qn, lang],
        ).expect("insert lineage");
    }

    // Insert function touches
    // Commit aaa111: touches parser.rs and lexer.rs
    insert_touch(conn, 1, "aaa111", "src/parser.rs", "parser::parse", "added");
    insert_touch(conn, 2, "aaa111", "src/lexer.rs", "lexer::tokenize", "added");

    // Commit bbb222: touches parser.rs and lexer.rs (same pair again — increases support)
    insert_touch(conn, 1, "bbb222", "src/parser.rs", "parser::parse", "modified");
    insert_touch(conn, 2, "bbb222", "src/lexer.rs", "lexer::tokenize", "modified");

    // Commit ccc333: touches parser.rs, formatter.rs, and utils.rs
    insert_touch(conn, 1, "ccc333", "src/parser.rs", "parser::parse", "modified");
    insert_touch(conn, 3, "ccc333", "src/formatter.rs", "formatter::format", "added");
    insert_touch(conn, 4, "ccc333", "src/utils.rs", "utils::helper", "added");

    // Commit ddd444 (bot): touches parser.rs and utils.rs — should be filtered
    insert_touch(conn, 1, "ddd444", "src/parser.rs", "parser::parse", "modified");
    insert_touch(conn, 4, "ddd444", "src/utils.rs", "utils::helper", "modified");

    // Commit eee555 (fix): touches formatter.rs — follow-up fix after ccc333
    insert_touch(conn, 3, "eee555", "src/formatter.rs", "formatter::format", "modified");
}

fn insert_touch(
    conn: &rusqlite::Connection,
    lineage_id: u64,
    commit_hash: &str,
    path: &str,
    qualname: &str,
    change_type: &str,
) {
    conn.execute(
        "INSERT INTO function_touches (lineage_id, commit_hash, path, qualified_name, arity, change_type,
         signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, NULL, NULL, NULL, '', 'ok', 0, 0)",
        rusqlite::params![lineage_id, commit_hash, path, qualname, change_type],
    ).expect("insert touch");
}

// --- Tests ---

#[test]
fn test_commit_size_filtering() {
    let dir = tempfile::tempdir().unwrap();
    let engine = fresh_engine(&dir);
    let conn = engine.connection();

    // Insert a commit touching 30 files (above threshold)
    conn.execute(
        "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
         VALUES ('big111', 'dev', 'dev@x.com', 1700000000, 1, NULL, '', 'bulk reformat', 100, 50)",
        [],
    ).unwrap();

    for i in 0..30 {
        let path = format!("src/file_{i}.rs");
        conn.execute(
            "INSERT INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
             VALUES (?1, ?2, ?3, 'rust')",
            rusqlite::params![100 + i as i64, &path, format!("fn_{i}")],
        ).unwrap();
        insert_touch(conn, 100 + i as u64, "big111", &path, &format!("fn_{i}"), "added");
    }

    // Insert a small commit touching 3 files
    conn.execute(
        "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
         VALUES ('sml222', 'dev', 'dev@x.com', 1700100000, 2, NULL, '', 'small change', 5, 2)",
        [],
    ).unwrap();

    for i in 0..3 {
        let path = format!("src/small_{i}.rs");
        conn.execute(
            "INSERT INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
             VALUES (?1, ?2, ?3, 'rust')",
            rusqlite::params![200 + i as i64, &path, format!("sfn_{i}")],
        ).unwrap();
        insert_touch(conn, 200 + i as u64, "sml222", &path, &format!("sfn_{i}"), "added");
    }

    let config = CoChangeConfig::with_reference_epoch(1_700_200_000);
    let extractor = CoChangeExtractor::new(&engine, &config);
    let (file_pairs, _, stats) = extractor.extract().unwrap();

    assert_eq!(stats.total_commits, 2);
    assert_eq!(stats.size_filtered, 1, "30-file commit should be filtered");
    assert_eq!(stats.qualifying_commits, 1, "Only the 3-file commit should qualify");
    assert_eq!(file_pairs.len(), 3, "3 files produce C(3,2) = 3 pairs");
}

#[test]
fn test_bot_filtering() {
    let dir = tempfile::tempdir().unwrap();
    let engine = fresh_engine(&dir);
    seed_fixture(&engine);

    let config = CoChangeConfig::with_reference_epoch(1_700_400_000);
    let extractor = CoChangeExtractor::new(&engine, &config);
    let (_, _, stats) = extractor.extract().unwrap();

    assert_eq!(stats.bot_filtered, 1, "Bot commit ddd444 should be filtered");
    // Total = 5 commits, 1 bot filtered, 0 size filtered = 4 qualifying
    assert_eq!(stats.qualifying_commits, 4);
}

#[test]
fn test_metric_correctness() {
    let dir = tempfile::tempdir().unwrap();
    let engine = fresh_engine(&dir);
    seed_fixture(&engine);

    let config = CoChangeConfig::with_reference_epoch(1_700_400_000);
    let extractor = CoChangeExtractor::new(&engine, &config);
    let (file_pairs, _, stats) = extractor.extract().unwrap();

    let metrics = MetricsComputer::compute(&file_pairs, stats.qualifying_commits, &config);

    // parser.rs and lexer.rs co-change in commits aaa111 and bbb222 => support = 2
    let parser_lexer = metrics.iter().find(|m| {
        (m.entity_a == "src/lexer.rs" && m.entity_b == "src/parser.rs")
            || (m.entity_a == "src/parser.rs" && m.entity_b == "src/lexer.rs")
    });

    assert!(parser_lexer.is_some(), "parser-lexer pair should exist");
    let pl = parser_lexer.unwrap();
    assert_eq!(pl.support, 2, "parser-lexer support should be 2");
}

#[test]
fn test_graph_construction() {
    let dir = tempfile::tempdir().unwrap();
    let engine = fresh_engine(&dir);
    seed_fixture(&engine);

    let config = CoChangeConfig::with_reference_epoch(1_700_400_000);
    let extractor = CoChangeExtractor::new(&engine, &config);
    let (file_pairs, _, stats) = extractor.extract().unwrap();
    let metrics = MetricsComputer::compute(&file_pairs, stats.qualifying_commits, &config);
    let graph = CoChangeGraph::from_metrics(&metrics, EntityType::File);

    assert!(graph.node_count() >= 2, "Graph should have at least 2 nodes");
    assert!(graph.edge_count() >= 1, "Graph should have at least 1 edge");
    assert!(graph.contains("src/parser.rs"));

    let neighbors = graph.neighbors_of("src/parser.rs");
    assert!(!neighbors.is_empty(), "parser.rs should have neighbors");
    // lexer.rs should be the top coupled entity
    assert!(
        neighbors.iter().any(|n| n.name == "src/lexer.rs"),
        "lexer.rs should be a neighbor of parser.rs"
    );
}

#[test]
fn test_impact_scoring() {
    let dir = tempfile::tempdir().unwrap();
    let engine = fresh_engine(&dir);
    seed_fixture(&engine);

    let config = CoChangeConfig::with_reference_epoch(1_700_400_000);
    let extractor = CoChangeExtractor::new(&engine, &config);
    let (file_pairs, _, stats) = extractor.extract().unwrap();
    let metrics = MetricsComputer::compute(&file_pairs, stats.qualifying_commits, &config);
    let graph = CoChangeGraph::from_metrics(&metrics, EntityType::File);
    let analyzer = ImpactAnalyzer::new(&engine, &graph);

    let result = analyzer.analyze("src/parser.rs", 10);
    assert_eq!(result.target, "src/parser.rs");
    assert!(result.risk_score >= 0.0 && result.risk_score <= 1.0, "Risk score should be in [0, 1]");
    assert!(!result.coupled_entities.is_empty(), "Should have coupled entities");
}

#[test]
fn test_deterministic_extraction() {
    let dir = tempfile::tempdir().unwrap();
    let engine = fresh_engine(&dir);
    seed_fixture(&engine);

    let config = CoChangeConfig::with_reference_epoch(1_700_400_000);

    // Run extraction twice
    let extractor1 = CoChangeExtractor::new(&engine, &config);
    let (pairs1, _, stats1) = extractor1.extract().unwrap();

    let extractor2 = CoChangeExtractor::new(&engine, &config);
    let (pairs2, _, stats2) = extractor2.extract().unwrap();

    assert_eq!(stats1.qualifying_commits, stats2.qualifying_commits);
    assert_eq!(pairs1.len(), pairs2.len());

    // Verify metrics are identical
    let metrics1 = MetricsComputer::compute(&pairs1, stats1.qualifying_commits, &config);
    let metrics2 = MetricsComputer::compute(&pairs2, stats2.qualifying_commits, &config);

    assert_eq!(metrics1.len(), metrics2.len());
    for (m1, m2) in metrics1.iter().zip(metrics2.iter()) {
        assert_eq!(m1.entity_a, m2.entity_a);
        assert_eq!(m1.entity_b, m2.entity_b);
        assert_eq!(m1.support, m2.support);
    }
}
