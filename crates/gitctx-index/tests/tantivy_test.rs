use gitctx_index::tantivy_index::engine::TantivyEngine;
use gitctx_index::tantivy_index::incremental::IncrementalTantivyWriter;
use gitctx_index::tantivy_index::schema::TantivyTouchDoc;
use gitctx_index::tantivy_index::tokenizer::{tokenize_code, CodeTokenizer};
use std::time::Instant;
use tantivy::tokenizer::{TokenStream, Tokenizer};
use tempfile::tempdir;

#[test]
fn test_tantivy_code_tokenizer_and_bm25() {
    // 1. Test CodeTokenizer unit splitting
    let text = "getUserById get_user_by_id crate::auth::UserManager.authenticate getter indices parser";
    let mut tokenizer = CodeTokenizer;
    let mut stream = tokenizer.token_stream(text);

    let mut token_texts = Vec::new();
    while stream.advance() {
        token_texts.push(stream.token().text.clone());
    }

    // Assert camelCase tokens
    assert!(token_texts.contains(&"getUserById".to_string()));
    assert!(token_texts.contains(&"get".to_string()));
    assert!(token_texts.contains(&"user".to_string()));
    assert!(token_texts.contains(&"by".to_string()));
    assert!(token_texts.contains(&"id".to_string()));

    // Assert snake_case tokens
    assert!(token_texts.contains(&"get_user_by_id".to_string()));

    // Assert qualified path and class tokens
    assert!(token_texts.contains(&"crate::auth::UserManager.authenticate".to_string()));
    assert!(token_texts.contains(&"crate".to_string()));
    assert!(token_texts.contains(&"auth".to_string()));
    assert!(token_texts.contains(&"UserManager".to_string()));
    assert!(token_texts.contains(&"usermanager".to_string()));
    assert!(token_texts.contains(&"manager".to_string()));
    assert!(token_texts.contains(&"authenticate".to_string()));

    // Assert zero stemming on identifiers per D-05
    assert!(token_texts.contains(&"getter".to_string()));
    assert!(token_texts.contains(&"indices".to_string()));
    assert!(token_texts.contains(&"parser".to_string()));
    // Ensure "getter" did not produce a stemmed "get" at its offset
    let getter_tokens = tokenize_code("getter");
    assert_eq!(getter_tokens.len(), 1);
    assert_eq!(getter_tokens[0].text, "getter");

    let indices_tokens = tokenize_code("indices");
    assert_eq!(indices_tokens.len(), 1);
    assert_eq!(indices_tokens[0].text, "indices");

    // 2. Test Tantivy BM25 in-RAM search with CodeTokenizer
    let mut engine = TantivyEngine::create_in_ram().expect("create in ram");

    let docs = vec![
        TantivyTouchDoc {
            touch_id: 101,
            lineage_id: 1,
            commit_hash: "c1111".to_string(),
            qualified_name: "auth::getUserById".to_string(),
            path: "crates/auth/src/user.rs".to_string(),
            commit_message: "Implement user lookup by id".to_string(),
            chunk_text: "fn getUserById(id: u64) -> Option<User> { ... }".to_string(),
        },
        TantivyTouchDoc {
            touch_id: 102,
            lineage_id: 2,
            commit_hash: "c2222".to_string(),
            qualified_name: "billing::process_invoice_payment".to_string(),
            path: "crates/billing/src/invoice.rs".to_string(),
            commit_message: "Invoice payment processing".to_string(),
            chunk_text: "fn process_invoice_payment(invoice: Invoice) { ... }".to_string(),
        },
        TantivyTouchDoc {
            touch_id: 103,
            lineage_id: 3,
            commit_hash: "c3333".to_string(),
            qualified_name: "getter::get_value".to_string(),
            path: "crates/core/src/getter.rs".to_string(),
            commit_message: "Add getter helpers".to_string(),
            chunk_text: "fn get_value() -> i32 { 42 }".to_string(),
        },
    ];

    engine.add_touches(&docs).expect("add touches");
    engine.commit().expect("commit");

    // Search for camelCase subtoken "getUserById"
    let results = engine.search("getUserById", 10).expect("search");
    assert!(!results.is_empty());
    assert_eq!(results[0].0, 101);

    // Search for subword "user"
    let results_user = engine.search("user", 10).expect("search user");
    assert!(!results_user.is_empty());
    assert_eq!(results_user[0].0, 101);

    // Search for snake_case subtoken "invoice"
    let results_inv = engine.search("invoice", 10).expect("search invoice");
    assert!(!results_inv.is_empty());
    assert_eq!(results_inv[0].0, 102);

    // Search for exact word "getter" vs "get"
    let results_getter = engine.search("getter", 10).expect("search getter");
    assert!(!results_getter.is_empty());
    assert_eq!(results_getter[0].0, 103);
}

#[test]
fn test_tantivy_multi_field_boosts() {
    let temp_dir = tempdir().expect("tempdir");
    let tantivy_tmp = temp_dir.path().join("tantivy.tmp");
    let tantivy_final = temp_dir.path().join("tantivy");

    let mut engine =
        TantivyEngine::create_in_dir(&tantivy_tmp, 32_000_000).expect("create in dir");

    // Setup documents where matching term "authenticate" appears in different fields:
    // Doc 1: "authenticate" in qualified_name (boost 4.0)
    // Doc 2: "authenticate" in path (boost 2.0)
    // Doc 3: "authenticate" in commit_message (boost 1.2)
    // Doc 4: "authenticate" in chunk_text (boost 1.0)
    let docs = vec![
        TantivyTouchDoc {
            touch_id: 1,
            lineage_id: 1,
            commit_hash: "h1".to_string(),
            qualified_name: "auth::authenticate".to_string(),
            path: "src/service.rs".to_string(),
            commit_message: "routine update".to_string(),
            chunk_text: "fn run() {}".to_string(),
        },
        TantivyTouchDoc {
            touch_id: 2,
            lineage_id: 2,
            commit_hash: "h2".to_string(),
            qualified_name: "auth::check_credentials".to_string(),
            path: "src/authenticate.rs".to_string(),
            commit_message: "routine update".to_string(),
            chunk_text: "fn run() {}".to_string(),
        },
        TantivyTouchDoc {
            touch_id: 3,
            lineage_id: 3,
            commit_hash: "h3".to_string(),
            qualified_name: "auth::validate".to_string(),
            path: "src/service.rs".to_string(),
            commit_message: "add authenticate routine".to_string(),
            chunk_text: "fn run() {}".to_string(),
        },
        TantivyTouchDoc {
            touch_id: 4,
            lineage_id: 4,
            commit_hash: "h4".to_string(),
            qualified_name: "auth::verify".to_string(),
            path: "src/service.rs".to_string(),
            commit_message: "routine update".to_string(),
            chunk_text: "fn run() { authenticate(); }".to_string(),
        },
    ];

    engine.add_touches(&docs).expect("add touches");
    engine.commit().expect("commit");

    // Verify search ordering respects boosts:
    // qualified_name (4.0) > path (2.0) > commit_message (1.2) > chunk_text (1.0)
    let results = engine.search("authenticate", 10).expect("search authenticate");
    assert_eq!(results.len(), 4);

    let doc_order: Vec<u64> = results.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        doc_order,
        vec![1, 2, 3, 4],
        "Expected ranking order [1, 2, 3, 4] matching boosts 4.0 > 2.0 > 1.2 > 1.0"
    );

    // Verify scores are strictly decreasing
    assert!(results[0].1 > results[1].1);
    assert!(results[1].1 > results[2].1);
    assert!(results[2].1 > results[3].1);

    // Test atomic directory swap from .tmp to final target per D-07
    engine
        .commit_and_swap_to(&tantivy_final)
        .expect("atomic swap");

    assert!(!tantivy_tmp.exists(), "tmp directory should be moved");
    assert!(tantivy_final.exists(), "final directory should exist");

    // Open index from final directory and verify search continues to work
    let swapped_engine = TantivyEngine::open(&tantivy_final).expect("open swapped");
    let swapped_results = swapped_engine
        .search("authenticate", 10)
        .expect("search after swap");
    assert_eq!(swapped_results.len(), 4);
    assert_eq!(swapped_results[0].0, 1);
}

#[test]
fn test_tantivy_incremental_append() {
    let temp_dir = tempdir().expect("tempdir");
    let index_dir = temp_dir.path().join("tantivy");

    // Phase 1: Initialize index with initial base batch
    {
        let mut engine =
            TantivyEngine::create_in_dir(&index_dir, 32_000_000).expect("create in dir");
        let initial_doc = TantivyTouchDoc {
            touch_id: 1,
            lineage_id: 1,
            commit_hash: "init001".to_string(),
            qualified_name: "core::init_engine".to_string(),
            path: "crates/core/src/init.rs".to_string(),
            commit_message: "Initialize system engine".to_string(),
            chunk_text: "fn init_engine() -> Result<()> { ... }".to_string(),
        };
        engine.add_touch(&initial_doc).expect("add initial touch");
        engine.commit().expect("commit");
    }

    // Phase 2: Open IncrementalTantivyWriter and append new document
    let start_time = Instant::now();
    {
        let mut inc_writer =
            IncrementalTantivyWriter::open(&index_dir).expect("open incremental writer");

        let new_doc = TantivyTouchDoc {
            touch_id: 2,
            lineage_id: 2,
            commit_hash: "inc002".to_string(),
            qualified_name: "core::hot_reload_pipeline".to_string(),
            path: "crates/core/src/reload.rs".to_string(),
            commit_message: "Add hot reload pipeline".to_string(),
            chunk_text: "fn hot_reload_pipeline() { trigger(); }".to_string(),
        };

        inc_writer.add_touch(&new_doc).expect("add incremental touch");
        let _opstamp = inc_writer.finish().expect("finish incremental commit");
    }
    let elapsed = start_time.elapsed();
    assert!(
        elapsed.as_secs() < 2,
        "Incremental append and commit must finish in <2s per D-08, took {:?}",
        elapsed
    );

    // Phase 3: Verify reader sees both original and incrementally added documents
    let engine = TantivyEngine::open(&index_dir).expect("open index");

    // Search for original document
    let res1 = engine.search("init_engine", 10).expect("search init");
    assert!(!res1.is_empty());
    assert_eq!(res1[0].0, 1);

    // Search for incrementally added document
    let res2 = engine.search("hot_reload_pipeline", 10).expect("search reload");
    assert!(!res2.is_empty());
    assert_eq!(res2[0].0, 2);

    // Search for common word matching both
    let res_all = engine.search("core", 10).expect("search core");
    assert_eq!(res_all.len(), 2);
}
