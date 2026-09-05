use clap::Parser;
use gitctx_cli::commands::blame::BlameHunkInfo;
use gitctx_cli::commands::doctor::{self, DoctorCheck};
use gitctx_cli::envelope::ResponseEnvelope;
use gitctx_cli::format::{
    format_blame, format_doctor, format_evolution, format_impact_global, format_impact_single,
    format_search, format_show,
};
use gitctx_cli::{Cli, Format};
use gitctx_core::model::Citation;
use gitctx_index::storage::TouchRecord;
use gitctx_query::chain::{LineageChain, LineageChainNode};
use gitctx_query::cochange::{
    CoChangeMetrics, CoupledEntity, EntityType, ExtractionStats, ImpactResult,
};
use gitctx_query::retrieval::RetrievedFact;
use std::time::Duration;

#[test]
fn test_cli_parser_defaults_and_flags() {
    let cli = Cli::parse_from(["gitctx", "doctor"]);
    assert_eq!(cli.format, Format::Table);

    let cli_json = Cli::parse_from(["gitctx", "--format", "json", "doctor"]);
    assert_eq!(cli_json.format, Format::Json);

    let cli_search = Cli::parse_from(["gitctx", "search", "test query"]);
    match cli_search.command {
        Some(gitctx_cli::commands::Commands::Search { query }) => {
            assert_eq!(query, "test query");
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_response_envelope_serialization_roundtrip() {
    let data = serde_json::json!({
        "status": "ok",
        "value": 42
    });
    let env = ResponseEnvelope::success(data.clone());
    assert_eq!(env.version, "v1");
    assert_eq!(env.status, "success");
    assert!(env.error.is_none());

    let json_str = serde_json::to_string(&env).expect("Serialization failed");
    let deserialized: ResponseEnvelope<serde_json::Value> =
        serde_json::from_str(&json_str).expect("Deserialization failed");

    assert_eq!(deserialized.version, "v1");
    assert_eq!(deserialized.status, "success");
    assert_eq!(deserialized.data, Some(data));

    let err_env: ResponseEnvelope<()> =
        ResponseEnvelope::error("NOT_FOUND", "Item was not found in index");
    assert_eq!(err_env.status, "error");
    assert_eq!(
        err_env.error.unwrap().message,
        "Item was not found in index"
    );
}

#[test]
fn test_doctor_command_json_and_table() {
    let res_table = doctor::run(Format::Table);
    assert!(res_table.is_ok());

    let res_json = doctor::run(Format::Json);
    assert!(res_json.is_ok());
}

#[test]
fn test_table_formatters_smoke() {
    // Test format_doctor
    let checks = vec![
        DoctorCheck {
            name: "Git Repository Discovery".to_string(),
            status: "PASS".to_string(),
            details: "Repository discovered".to_string(),
        },
        DoctorCheck {
            name: "SQLite Index Database".to_string(),
            status: "WARN".to_string(),
            details: "No index created yet".to_string(),
        },
    ];
    format_doctor(&checks);

    // Test format_evolution
    let chain = LineageChain {
        lineage_id: 1,
        current_qualified_name: "test::my_func".to_string(),
        current_path: "src/lib.rs".to_string(),
        language: "rust".to_string(),
        nodes: vec![LineageChainNode {
            touch: TouchRecord {
                touch_id: Some(1),
                lineage_id: 1,
                commit_hash: "abc12345678".to_string(),
                path: "src/lib.rs".to_string(),
                qualified_name: "test::my_func".to_string(),
                arity: 0,
                change_type: "insert".to_string(),
                signature_before: None,
                signature_after: Some("fn my_func()".to_string()),
                fingerprint: None,
                chunk_text: "fn my_func() {}".to_string(),
                parse_status: "ok".to_string(),
                start_line: 10,
                end_line: 20,
            },
            commit_author: "Developer".to_string(),
            commit_authored_at: 1700000000,
            topo_order: 1,
            commit_message: "feat: initial implementation".to_string(),
            citation: Citation {
                commit_hash: "abc12345678".to_string(),
                path: "src/lib.rs".to_string(),
                start_line: 10,
                end_line: 20,
                match_rule: "symbol_boost".to_string(),
                match_confidence: 0.95,
            },
        }],
    };
    format_evolution(&chain);

    // Test format_search
    let results = vec![RetrievedFact {
        touch_id: 1,
        lineage_id: 1,
        commit_hash: "abc12345678".to_string(),
        qualified_name: "test::my_func".to_string(),
        path: "src/lib.rs".to_string(),
        change_type: "modify".to_string(),
        chunk_text: "fn my_func() {}".to_string(),
        score: 0.85,
        rank: 1,
        citation: Citation {
            commit_hash: "abc12345678".to_string(),
            path: "src/lib.rs".to_string(),
            start_line: 10,
            end_line: 20,
            match_rule: "symbol_boost".to_string(),
            match_confidence: 0.95,
        },
    }];
    format_search("my_func", &results, Duration::from_millis(5));

    // Test format_impact
    let impact = ImpactResult {
        target: "src/lib.rs".to_string(),
        entity_type: EntityType::File,
        risk_score: 0.75,
        followup_fix_count: 2,
        coupled_entities: vec![CoupledEntity {
            name: "src/util.rs".to_string(),
            entity_type: EntityType::File,
            confidence: 0.8,
            lift: 2.5,
            support: 12,
            weighted_support: 11.5,
        }],
    };
    format_impact_single("src/lib.rs", &impact);

    let stats = ExtractionStats {
        total_commits: 100,
        qualifying_commits: 80,
        size_filtered: 15,
        bot_filtered: 5,
        total_file_pairs: 50,
        total_function_pairs: 30,
    };
    let file_metrics = vec![CoChangeMetrics {
        entity_a: "src/a.rs".to_string(),
        entity_b: "src/b.rs".to_string(),
        support: 10,
        weighted_support: 9.5,
        confidence_a_to_b: 0.8,
        confidence_b_to_a: 0.8,
        lift: 2.0,
        total_qualifying_commits: 80,
    }];
    format_impact_global(&stats, &file_metrics, &[]);

    // Test format_show
    let touches = vec![(
        "src/lib.rs".to_string(),
        "my_func".to_string(),
        "modify".to_string(),
        10,
        25,
    )];
    format_show(
        "abc12345678",
        "Author",
        "author@example.com",
        1700000000,
        "fix: bug fix",
        "parent123",
        &touches,
    );

    // Test format_blame
    let hunks = vec![BlameHunkInfo {
        commit: "abc12345678".to_string(),
        author: "Author".to_string(),
        start_line: 1,
        line_count: 10,
    }];
    format_blame("src/lib.rs", &hunks);
}
