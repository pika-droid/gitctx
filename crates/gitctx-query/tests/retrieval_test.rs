#![allow(clippy::cast_possible_wrap)]

use gitctx_index::storage::{initialize_schema, open_connection, CommitRecord, TouchRecord};
use gitctx_index::tantivy_index::TantivyEngine;
use gitctx_query::chain::{ChainAssembler, LineageChain};
use gitctx_query::citation::CitationBuilder;
use gitctx_query::retrieval::{
    deduplicate_and_cap_touches, HybridRetriever, RetrievedFact, DEFAULT_MAX_TOUCHES_PER_COMMIT,
};
use gitctx_query::rrf::{fuse_rankings, RrfParams, RRF_DEFAULT_K};
use gitctx_query::symbol::ExactSymbolMatcher;
use rusqlite::params;

fn setup_test_sqlite() -> rusqlite::Connection {
    let conn = open_connection(":memory:").expect("Open in-memory db");
    initialize_schema(&conn).expect("Initialize schema");
    conn
}

fn seed_test_database(conn: &rusqlite::Connection) {
    // Insert commits with distinct topo_order
    let commits = vec![
        CommitRecord {
            hash: "commit_aaa".to_string(),
            author: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            authored_at: 1000,
            topo_order: 1,
            branch: Some("main".to_string()),
            parent_hashes: String::new(),
            message: "Initial commit".to_string(),
            insertions: 100,
            deletions: 0,
        },
        CommitRecord {
            hash: "commit_bbb".to_string(),
            author: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            authored_at: 2000,
            topo_order: 2,
            branch: Some("main".to_string()),
            parent_hashes: "commit_aaa".to_string(),
            message: "Refactor auth and utilities".to_string(),
            insertions: 50,
            deletions: 10,
        },
        CommitRecord {
            hash: "commit_ccc".to_string(),
            author: "Charlie".to_string(),
            email: "charlie@example.com".to_string(),
            authored_at: 3000,
            topo_order: 3,
            branch: Some("main".to_string()),
            parent_hashes: "commit_bbb".to_string(),
            message: "Update UserAuth login logic".to_string(),
            insertions: 20,
            deletions: 5,
        },
    ];

    for c in &commits {
        conn.execute(
            "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);",
            params![c.hash, c.author, c.email, c.authored_at, c.topo_order, c.branch, c.parent_hashes, c.message, c.insertions, c.deletions],
        ).expect("Insert commit");
    }

    // Insert function lineages
    conn.execute(
        "INSERT INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
         VALUES (1, 'src/auth.rs', 'UserAuth::login', 'rust'),
                (2, 'src/util.rs', 'format_date', 'rust');",
        [],
    )
    .expect("Insert lineages");

    // Insert function touches
    let touches = vec![
        // Lineage 1 touches across commits aaa, bbb, ccc
        TouchRecord {
            touch_id: Some(101),
            lineage_id: 1,
            commit_hash: "commit_aaa".to_string(),
            path: "src/auth.rs".to_string(),
            qualified_name: "UserAuth::login".to_string(),
            arity: 2,
            change_type: "added".to_string(),
            signature_before: None,
            signature_after: Some("fn login(&self, u: &str)".to_string()),
            fingerprint: None,
            chunk_text: "fn login(&self, u: &str) { println!(\"login\"); }".to_string(),
            parse_status: "ok".to_string(),
            start_line: 10,
            end_line: 20,
        },
        TouchRecord {
            touch_id: Some(102),
            lineage_id: 1,
            commit_hash: "commit_bbb".to_string(),
            path: "src/auth.rs".to_string(),
            qualified_name: "UserAuth::login".to_string(),
            arity: 2,
            change_type: "modified".to_string(),
            signature_before: Some("fn login(&self, u: &str)".to_string()),
            signature_after: Some("fn login(&self, u: &str, p: &str)".to_string()),
            fingerprint: None,
            chunk_text: "fn login(&self, u: &str, p: &str) { verify(u, p); }".to_string(),
            parse_status: "ok".to_string(),
            start_line: 10,
            end_line: 25,
        },
        TouchRecord {
            touch_id: Some(103),
            lineage_id: 1,
            commit_hash: "commit_ccc".to_string(),
            path: "src/auth.rs".to_string(),
            qualified_name: "UserAuth::login".to_string(),
            arity: 2,
            change_type: "modified".to_string(),
            signature_before: Some("fn login(&self, u: &str, p: &str)".to_string()),
            signature_after: Some("fn login(&self, u: &str, p: &str) -> bool".to_string()),
            fingerprint: None,
            chunk_text: "fn login(&self, u: &str, p: &str) -> bool { verify(u, p) }".to_string(),
            parse_status: "ok".to_string(),
            start_line: 12,
            end_line: 28,
        },
        // Lineage 2 touch
        TouchRecord {
            touch_id: Some(201),
            lineage_id: 2,
            commit_hash: "commit_bbb".to_string(),
            path: "src/util.rs".to_string(),
            qualified_name: "format_date".to_string(),
            arity: 1,
            change_type: "added".to_string(),
            signature_before: None,
            signature_after: Some("fn format_date(t: i64) -> String".to_string()),
            fingerprint: None,
            chunk_text: "fn format_date(t: i64) -> String { t.to_string() }".to_string(),
            parse_status: "ok".to_string(),
            start_line: 1,
            end_line: 10,
        },
    ];

    for t in &touches {
        conn.execute(
            "INSERT INTO function_touches (touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type, signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14);",
            params![
                t.touch_id,
                t.lineage_id,
                t.commit_hash,
                t.path,
                t.qualified_name,
                t.arity,
                t.change_type,
                t.signature_before,
                t.signature_after,
                t.fingerprint,
                t.chunk_text,
                t.parse_status,
                t.start_line as i64,
                t.end_line as i64,
            ],
        ).expect("Insert touch");
    }
}

#[test]
fn test_rrf_hybrid_fusion_and_exact_symbol_fastpath() {
    let conn = setup_test_sqlite();
    seed_test_database(&conn);

    // 1. Symbol Heuristic Detection
    assert!(ExactSymbolMatcher::is_symbol_candidate("UserAuth::login"));
    assert!(ExactSymbolMatcher::is_symbol_candidate("handle_request"));
    assert!(ExactSymbolMatcher::is_symbol_candidate("std.io.read"));
    assert!(!ExactSymbolMatcher::is_symbol_candidate(
        "how to fix login bug"
    ));
    assert!(!ExactSymbolMatcher::is_symbol_candidate("   "));

    // 2. Exact Symbol Fast-Path SQLite lookup
    let exact_hits = ExactSymbolMatcher::find_exact_symbol_touches(&conn, "UserAuth::login", 10)
        .expect("Find exact symbol touches");
    assert_eq!(exact_hits.len(), 3);
    assert!(exact_hits.contains(&101));
    assert!(exact_hits.contains(&102));
    assert!(exact_hits.contains(&103));

    // 3. RRF Fusion math verification
    // Candidate lists:
    // exact: [103] -> injected at rank 0: 1.0 / (60.0 + 0) = 0.0166667
    // bm25:  [201, 103] -> 201 gets 1/(60+0)=0.0166667, 103 gets 1/(60+1)=0.0163934
    // dense: [102, 201] -> 102 gets 1/(60+0)=0.0166667, 201 gets 1/(60+1)=0.0163934
    // Expected combined scores:
    // 103: exact(rank 0) + bm25(rank 1) = 0.0166667 + 0.0163934 = 0.0330601
    // 201: bm25(rank 0) + dense(rank 1) = 0.0166667 + 0.0163934 = 0.0330601
    // 102: dense(rank 0) = 0.0166667
    let exact_list = vec![103];
    let bm25_list = vec![201, 103];
    let dense_list = vec![102, 201];
    let params = RrfParams::new(RRF_DEFAULT_K);

    let fused = fuse_rankings(&exact_list, &bm25_list, &dense_list, &params);
    assert_eq!(fused.len(), 3);

    // 103 and 201 have tied top scores; tie broken by touch_id ASC -> 103 before 201
    assert_eq!(fused[0].touch_id, 103);
    assert!(fused[0].is_exact_match);
    assert_eq!(fused[1].touch_id, 201);
    assert_eq!(fused[2].touch_id, 102);

    // Verify custom RRF k parameter (e.g. k=30)
    let params_k30 = RrfParams::new(30.0);
    let fused_k30 = fuse_rankings(&exact_list, &bm25_list, &dense_list, &params_k30);
    let expected_103_k30 = (1.0 / 30.0) + (1.0 / 31.0);
    let diff = (fused_k30[0].score - expected_103_k30).abs();
    assert!(diff < 1e-6);

    // 4. End-to-end HybridRetriever search with exact symbol fast-path
    let tantivy = TantivyEngine::create_in_ram().expect("Create in-ram tantivy");
    let retriever = HybridRetriever::new(&conn, Some(&tantivy), None);
    let results = retriever
        .search("UserAuth::login", None, 10)
        .expect("Search");

    assert!(!results.is_empty());
    // Exact symbol candidate should surface at Rank 0
    assert_eq!(results[0].qualified_name, "UserAuth::login");
    assert_eq!(results[0].rank, 0);
    assert_eq!(results[0].citation.match_rule, "exact_symbol");
    assert_eq!(results[0].citation.match_confidence, 1.0);
}

#[test]
fn test_deduplication_and_density_capping() {
    let commit_hash = "commit_refactor".to_string();
    let duplicate_chunk = "fn boilerplate() { /* identical boilerplate text */ }".to_string();

    let mut candidates = Vec::new();

    // Add 5 touches for the same commit, 3 of which have identical chunk text
    for i in 1..=5 {
        let chunk_text = if i <= 3 {
            duplicate_chunk.clone()
        } else {
            format!("fn unique_function_{i}() {{ work({i}); }}")
        };

        let touch = TouchRecord {
            touch_id: Some(i),
            lineage_id: i as u64,
            commit_hash: commit_hash.clone(),
            path: format!("src/module_{i}.rs"),
            qualified_name: format!("fn_{i}"),
            arity: 0,
            change_type: "modified".to_string(),
            signature_before: None,
            signature_after: None,
            fingerprint: None,
            chunk_text,
            parse_status: "ok".to_string(),
            start_line: 10,
            end_line: 20,
        };

        // Decreasing score
        let score = 1.0 - (i as f64 * 0.1);
        candidates.push((touch, score, "bm25".to_string(), 0.9f32));
    }

    // Add 2 touches from a different commit
    for i in 6..=7 {
        let touch = TouchRecord {
            touch_id: Some(i),
            lineage_id: i as u64,
            commit_hash: "commit_other".to_string(),
            path: format!("src/other_{i}.rs"),
            qualified_name: format!("other_fn_{i}"),
            arity: 0,
            change_type: "added".to_string(),
            signature_before: None,
            signature_after: None,
            fingerprint: None,
            chunk_text: format!("fn other_{i}() {{}}"),
            parse_status: "ok".to_string(),
            start_line: 5,
            end_line: 15,
        };
        let score = 0.4 - (i as f64 * 0.05);
        candidates.push((touch, score, "bm25".to_string(), 0.8f32));
    }

    // Test deduplication and capping with max 3 touches per commit
    let filtered = deduplicate_and_cap_touches(candidates, DEFAULT_MAX_TOUCHES_PER_COMMIT, 10)
        .expect("Deduplicate and cap");

    // Check duplicate suppression:
    // Out of touch 1, 2, 3 (all identical boilerplate chunk), only touch 1 (highest score) should be kept.
    let touch_ids: Vec<u64> = filtered.iter().map(|f| f.touch_id).collect();
    assert!(touch_ids.contains(&1));
    assert!(!touch_ids.contains(&2));
    assert!(!touch_ids.contains(&3));

    // Check density capping:
    // For 'commit_refactor', touch 1, 4, 5 are kept (total 3).
    let refactor_touches: Vec<&RetrievedFact> = filtered
        .iter()
        .filter(|f| f.commit_hash == "commit_refactor")
        .collect();
    assert_eq!(refactor_touches.len(), 3);
    assert_eq!(refactor_touches.len(), DEFAULT_MAX_TOUCHES_PER_COMMIT);

    // Other commit touches are preserved
    let other_touches: Vec<&RetrievedFact> = filtered
        .iter()
        .filter(|f| f.commit_hash == "commit_other")
        .collect();
    assert_eq!(other_touches.len(), 2);
}

#[test]
fn test_deterministic_chain_assembly() {
    let conn = setup_test_sqlite();
    seed_test_database(&conn);

    // Assemble chronological lineage chain for lineage 1 (UserAuth::login)
    let chain: LineageChain =
        ChainAssembler::assemble_lineage_chain(&conn, 1).expect("Assemble lineage chain");

    assert_eq!(chain.lineage_id, 1);
    assert_eq!(chain.current_qualified_name, "UserAuth::login");
    assert_eq!(chain.current_path, "src/auth.rs");
    assert_eq!(chain.language, "rust");

    // Verify exactly 3 evolutionary nodes in chronological topo_order ASC
    assert_eq!(chain.nodes.len(), 3);

    // Node 0: Initial commit (topo_order: 1)
    assert_eq!(chain.nodes[0].topo_order, 1);
    assert_eq!(chain.nodes[0].touch.commit_hash, "commit_aaa");
    assert_eq!(chain.nodes[0].commit_author, "Alice");
    assert_eq!(chain.nodes[0].touch.change_type, "added");
    assert_eq!(chain.nodes[0].touch.start_line, 10);
    assert_eq!(chain.nodes[0].touch.end_line, 20);

    // Node 1: Second commit (topo_order: 2)
    assert_eq!(chain.nodes[1].topo_order, 2);
    assert_eq!(chain.nodes[1].touch.commit_hash, "commit_bbb");
    assert_eq!(chain.nodes[1].commit_author, "Bob");
    assert_eq!(chain.nodes[1].touch.change_type, "modified");

    // Node 2: Third commit (topo_order: 3)
    assert_eq!(chain.nodes[2].topo_order, 3);
    assert_eq!(chain.nodes[2].touch.commit_hash, "commit_ccc");
    assert_eq!(chain.nodes[2].commit_author, "Charlie");
    assert_eq!(chain.nodes[2].touch.change_type, "modified");

    // Verify ordering is strictly ascending
    assert!(chain.nodes[0].topo_order < chain.nodes[1].topo_order);
    assert!(chain.nodes[1].topo_order < chain.nodes[2].topo_order);
}

#[test]
fn test_citation_payload_completeness() {
    // 1. Valid citation construction
    let citation = CitationBuilder::build(
        "abc1234567890",
        "src/core/auth.rs",
        15,
        45,
        "rrf_bm25_dense",
        0.95,
    )
    .expect("Build citation");

    assert_eq!(citation.commit_hash, "abc1234567890");
    assert_eq!(citation.path, "src/core/auth.rs");
    assert_eq!(citation.start_line, 15);
    assert_eq!(citation.end_line, 45);
    assert_eq!(citation.match_rule, "rrf_bm25_dense");
    assert_eq!(citation.match_confidence, 0.95);

    // 2. Invariant I4 enforcement: invalid empty commit_hash, path, or rule fails
    assert!(CitationBuilder::build("", "src/main.rs", 1, 10, "bm25", 0.5).is_err());
    assert!(CitationBuilder::build("abc1234", "", 1, 10, "bm25", 0.5).is_err());
    assert!(CitationBuilder::build("abc1234", "src/main.rs", 1, 10, "", 0.5).is_err());

    // 3. Confidence clamping
    let clamped_high =
        CitationBuilder::build("hash", "path.rs", 1, 10, "rule", 1.5).expect("Clamped high");
    assert_eq!(clamped_high.match_confidence, 1.0);

    let clamped_low =
        CitationBuilder::build("hash", "path.rs", 1, 10, "rule", -0.5).expect("Clamped low");
    assert_eq!(clamped_low.match_confidence, 0.0);

    // 4. Citation attachment on retrieved search facts and lineage nodes
    let conn = setup_test_sqlite();
    seed_test_database(&conn);
    let chain = ChainAssembler::assemble_lineage_chain(&conn, 1).expect("Assemble chain");
    for node in chain.nodes {
        assert!(!node.citation.commit_hash.is_empty());
        assert!(!node.citation.path.is_empty());
        assert!(node.citation.end_line >= node.citation.start_line);
        assert!(!node.citation.match_rule.is_empty());
        assert!(node.citation.match_confidence >= 0.0 && node.citation.match_confidence <= 1.0);
    }
}
