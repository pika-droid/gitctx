use gitctx_core::error::CoreError;
use gitctx_core::git::{CommitDiff, DeltaStatus, FileDelta};
use gitctx_core::lineage::{DerivationKind, LineageDag, LineageEdge, LineageResolver};
use gitctx_core::model::{CommitId, LineageId, ParseStatus};
use gitctx_core::parser::{ParsedFileSymbols, RawFunctionSymbol};
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_lineage_dag_basic_traversal() {
    let mut dag = LineageDag::new();
    let c1 = CommitId::new("commit_c1");
    let c2 = CommitId::new("commit_c2");

    let l1 = LineageId::new(1);
    let l2 = LineageId::new(2);
    let l3 = LineageId::new(3);

    let edge1 = LineageEdge {
        parent_lineage_id: l1,
        child_lineage_id: l2,
        commit_hash: c1.clone(),
        kind: DerivationKind::Split,
        similarity: 0.65,
    };
    let edge2 = LineageEdge {
        parent_lineage_id: l2,
        child_lineage_id: l3,
        commit_hash: c2.clone(),
        kind: DerivationKind::Copy,
        similarity: 0.90,
    };

    dag.add_edge(edge1);
    dag.add_edge(edge2);

    assert_eq!(dag.len(), 2);
    assert!(!dag.is_empty());

    // Children query
    let children_l1 = dag.get_children(l1);
    assert_eq!(children_l1.len(), 1);
    assert_eq!(children_l1[0].child_lineage_id, l2);

    // Parents query
    let parents_l3 = dag.get_parents(l3);
    assert_eq!(parents_l3.len(), 1);
    assert_eq!(parents_l3[0].parent_lineage_id, l2);

    // Commit query
    let edges_c1 = dag.get_edges_for_commit(&c1);
    assert_eq!(edges_c1.len(), 1);
    assert_eq!(edges_c1[0].kind, DerivationKind::Split);

    // Ancestor / Descendant traversal
    let ancestors_l3 = dag.ancestors(l3);
    assert!(ancestors_l3.contains(&l1));
    assert!(ancestors_l3.contains(&l2));

    let descendants_l1 = dag.descendants(l1);
    assert!(descendants_l1.contains(&l2));
    assert!(descendants_l1.contains(&l3));
}

#[test]
fn test_split_derivation_m7() {
    let mut resolver = LineageResolver::new();

    // Commit 1: Parent monolithic function containing calculation logic
    let c1_id = CommitId::new("commit_c1");
    let c1_code = b"fn process_orders(orders: &[Order]) {\n    for o in orders {\n        verify_cart(o);\n        let tax = o.amount * 0.10;\n        let surcharge = tax * 0.05;\n        let discount = if o.amount > 100.0 { 10.0 } else { 0.0 };\n        let total = o.amount + tax + surcharge - discount;\n        save_to_db(total);\n        send_notification(o, total);\n        audit_log(o, total);\n    }\n}\n";
    let mut c1_symbols = HashMap::new();
    c1_symbols.insert(
        PathBuf::from("src/orders.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "process_orders".to_string(),
                qualname: "process_orders".to_string(),
                signature: "fn process_orders(orders: &[Order])".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c1_code.len(),
                start_line: 1,
                end_line: 12,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c1_contents = HashMap::new();
    c1_contents.insert(PathBuf::from("src/orders.rs"), c1_code.to_vec());

    let c1_diff = CommitDiff {
        commit_id: c1_id.clone(),
        parent_id: None,
        deltas: vec![FileDelta {
            old_path: None,
            new_path: Some(PathBuf::from("src/orders.rs")),
            status: DeltaStatus::Added,
            old_oid: None,
            new_oid: Some("oid1".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res1 = resolver
        .resolve_commit(&c1_id, &c1_diff, &c1_symbols, &c1_contents)
        .unwrap();
    assert_eq!(res1.touches.len(), 1);
    let parent_lineage = res1.touches[0].lineage_id;

    // Commit 2: Function Split (M7) - process_orders refactored, helper calculate_order_total extracted
    let c2_id = CommitId::new("commit_c2");
    let c2_code_proc = b"fn process_orders(orders: &[Order]) {\n    for o in orders {\n        verify_cart(o);\n        let total = calculate_order_total(o);\n        save_to_db(total);\n        send_notification(o, total);\n        audit_log(o, total);\n    }\n}\n";
    let c2_code_helper = b"fn calculate_order_total(o: &Order) -> f64 {\n    let tax = o.amount * 0.10;\n    let surcharge = tax * 0.05;\n    let discount = if o.amount > 100.0 { 10.0 } else { 0.0 };\n    let total = o.amount + tax + surcharge - discount;\n    total\n}\n";

    let mut c2_full_code = Vec::new();
    c2_full_code.extend_from_slice(c2_code_proc);
    let split_pos = c2_full_code.len();
    c2_full_code.extend_from_slice(c2_code_helper);

    let mut c2_symbols = HashMap::new();
    c2_symbols.insert(
        PathBuf::from("src/orders.rs"),
        ParsedFileSymbols {
            symbols: vec![
                RawFunctionSymbol {
                    name: "process_orders".to_string(),
                    qualname: "process_orders".to_string(),
                    signature: "fn process_orders(orders: &[Order])".to_string(),
                    docstring: None,
                    start_byte: 0,
                    end_byte: split_pos,
                    start_line: 1,
                    end_line: 7,
                    is_method: false,
                },
                RawFunctionSymbol {
                    name: "calculate_order_total".to_string(),
                    qualname: "calculate_order_total".to_string(),
                    signature: "fn calculate_order_total(o: &Order) -> f64".to_string(),
                    docstring: None,
                    start_byte: split_pos,
                    end_byte: c2_full_code.len(),
                    start_line: 8,
                    end_line: 15,
                    is_method: false,
                },
            ],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c2_contents = HashMap::new();
    c2_contents.insert(PathBuf::from("src/orders.rs"), c2_full_code);

    let c2_diff = CommitDiff {
        commit_id: c2_id.clone(),
        parent_id: Some(c1_id),
        deltas: vec![FileDelta {
            old_path: Some(PathBuf::from("src/orders.rs")),
            new_path: Some(PathBuf::from("src/orders.rs")),
            status: DeltaStatus::Modified,
            old_oid: Some("oid1".to_string()),
            new_oid: Some("oid2".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res2 = resolver
        .resolve_commit(&c2_id, &c2_diff, &c2_symbols, &c2_contents)
        .unwrap();
    assert_eq!(res2.touches.len(), 2);

    // Dominant child process_orders retains parent LineageId
    let proc_touch = res2
        .touches
        .iter()
        .find(|t| t.qualname == "process_orders")
        .unwrap();
    assert_eq!(proc_touch.lineage_id, parent_lineage);

    // Helper calculate_order_total receives fresh LineageId
    let helper_touch = res2
        .touches
        .iter()
        .find(|t| t.qualname == "calculate_order_total")
        .unwrap();
    assert_ne!(helper_touch.lineage_id, parent_lineage);

    // Derivation DAG records Split edge
    assert_eq!(res2.derivation_edges.len(), 1);
    let edge = &res2.derivation_edges[0];
    assert_eq!(edge.kind, DerivationKind::Split);
    assert_eq!(edge.parent_lineage_id, parent_lineage);
    assert_eq!(edge.child_lineage_id, helper_touch.lineage_id);
}

#[test]
fn test_merge_derivation() {
    let mut resolver = LineageResolver::new();

    // Commit 1: Two separate functions compute_part_a and compute_part_b
    let c1_id = CommitId::new("commit_c1");
    let c1_code = b"fn compute_part_a(x: i32) -> i32 {\n    let val = x * 2;\n    val + 10\n}\nfn compute_part_b(x: i32) -> i32 {\n    let val = x * 2;\n    val + 20\n}\n";
    let mut c1_symbols = HashMap::new();
    c1_symbols.insert(
        PathBuf::from("src/compute.rs"),
        ParsedFileSymbols {
            symbols: vec![
                RawFunctionSymbol {
                    name: "compute_part_a".to_string(),
                    qualname: "compute_part_a".to_string(),
                    signature: "fn compute_part_a(x: i32) -> i32".to_string(),
                    docstring: None,
                    start_byte: 0,
                    end_byte: 68,
                    start_line: 1,
                    end_line: 4,
                    is_method: false,
                },
                RawFunctionSymbol {
                    name: "compute_part_b".to_string(),
                    qualname: "compute_part_b".to_string(),
                    signature: "fn compute_part_b(x: i32) -> i32".to_string(),
                    docstring: None,
                    start_byte: 69,
                    end_byte: c1_code.len(),
                    start_line: 5,
                    end_line: 8,
                    is_method: false,
                },
            ],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c1_contents = HashMap::new();
    c1_contents.insert(PathBuf::from("src/compute.rs"), c1_code.to_vec());

    let c1_diff = CommitDiff {
        commit_id: c1_id.clone(),
        parent_id: None,
        deltas: vec![FileDelta {
            old_path: None,
            new_path: Some(PathBuf::from("src/compute.rs")),
            status: DeltaStatus::Added,
            old_oid: None,
            new_oid: Some("oid1".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res1 = resolver
        .resolve_commit(&c1_id, &c1_diff, &c1_symbols, &c1_contents)
        .unwrap();
    assert_eq!(res1.touches.len(), 2);
    let l_a = res1
        .touches
        .iter()
        .find(|t| t.qualname == "compute_part_a")
        .unwrap()
        .lineage_id;
    let l_b = res1
        .touches
        .iter()
        .find(|t| t.qualname == "compute_part_b")
        .unwrap()
        .lineage_id;

    // Commit 2: Merge both into compute_part_a
    let c2_id = CommitId::new("commit_c2");
    let c2_code =
        b"fn compute_part_a(x: i32) -> i32 {\n    let val = x * 2;\n    val + 10 + 20\n}\n";
    let mut c2_symbols = HashMap::new();
    c2_symbols.insert(
        PathBuf::from("src/compute.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "compute_part_a".to_string(),
                qualname: "compute_part_a".to_string(),
                signature: "fn compute_part_a(x: i32) -> i32".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c2_code.len(),
                start_line: 1,
                end_line: 4,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c2_contents = HashMap::new();
    c2_contents.insert(PathBuf::from("src/compute.rs"), c2_code.to_vec());

    let c2_diff = CommitDiff {
        commit_id: c2_id.clone(),
        parent_id: Some(c1_id),
        deltas: vec![FileDelta {
            old_path: Some(PathBuf::from("src/compute.rs")),
            new_path: Some(PathBuf::from("src/compute.rs")),
            status: DeltaStatus::Modified,
            old_oid: Some("oid1".to_string()),
            new_oid: Some("oid2".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res2 = resolver
        .resolve_commit(&c2_id, &c2_diff, &c2_symbols, &c2_contents)
        .unwrap();
    assert_eq!(res2.touches.len(), 1);
    assert_eq!(res2.touches[0].lineage_id, l_a);

    // Merge edge recorded for unmatched parent l_b -> l_a
    assert_eq!(res2.derivation_edges.len(), 1);
    let edge = &res2.derivation_edges[0];
    assert_eq!(edge.kind, DerivationKind::Merge);
    assert_eq!(edge.parent_lineage_id, l_b);
    assert_eq!(edge.child_lineage_id, l_a);
}

#[test]
fn test_copy_derivation_m10() {
    let mut resolver = LineageResolver::new();

    // Commit 1: Add original algorithm in src/algo.rs
    let c1_id = CommitId::new("commit_c1");
    let c1_code = b"fn run_algorithm(data: &[f64]) -> f64 {\n    let mut sum = 0.0;\n    for d in data {\n        sum += d * 1.5;\n    }\n    sum / data.len() as f64\n}\n";
    let mut c1_symbols = HashMap::new();
    c1_symbols.insert(
        PathBuf::from("src/algo.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "run_algorithm".to_string(),
                qualname: "run_algorithm".to_string(),
                signature: "fn run_algorithm(data: &[f64]) -> f64".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c1_code.len(),
                start_line: 1,
                end_line: 7,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c1_contents = HashMap::new();
    c1_contents.insert(PathBuf::from("src/algo.rs"), c1_code.to_vec());

    let c1_diff = CommitDiff {
        commit_id: c1_id.clone(),
        parent_id: None,
        deltas: vec![FileDelta {
            old_path: None,
            new_path: Some(PathBuf::from("src/algo.rs")),
            status: DeltaStatus::Added,
            old_oid: None,
            new_oid: Some("oid1".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res1 = resolver
        .resolve_commit(&c1_id, &c1_diff, &c1_symbols, &c1_contents)
        .unwrap();
    assert_eq!(res1.touches.len(), 1);
    let orig_lineage = res1.touches[0].lineage_id;

    // Commit 2: Function Copy (M10) - algo.rs stays intact, duplicated in src/algo_v2.rs
    let c2_id = CommitId::new("commit_c2");
    let mut c2_symbols = HashMap::new();
    c2_symbols.insert(
        PathBuf::from("src/algo.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "run_algorithm".to_string(),
                qualname: "run_algorithm".to_string(),
                signature: "fn run_algorithm(data: &[f64]) -> f64".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c1_code.len(),
                start_line: 1,
                end_line: 7,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    c2_symbols.insert(
        PathBuf::from("src/algo_v2.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "run_algorithm_v2".to_string(),
                qualname: "run_algorithm_v2".to_string(),
                signature: "fn run_algorithm_v2(data: &[f64]) -> f64".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c1_code.len(),
                start_line: 1,
                end_line: 7,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c2_contents = HashMap::new();
    c2_contents.insert(PathBuf::from("src/algo.rs"), c1_code.to_vec());
    c2_contents.insert(PathBuf::from("src/algo_v2.rs"), c1_code.to_vec());

    let c2_diff = CommitDiff {
        commit_id: c2_id.clone(),
        parent_id: Some(c1_id),
        deltas: vec![
            FileDelta {
                old_path: Some(PathBuf::from("src/algo.rs")),
                new_path: Some(PathBuf::from("src/algo.rs")),
                status: DeltaStatus::Modified,
                old_oid: Some("oid1".to_string()),
                new_oid: Some("oid1".to_string()),
                similarity: 100,
                is_binary: false,
            },
            FileDelta {
                old_path: None,
                new_path: Some(PathBuf::from("src/algo_v2.rs")),
                status: DeltaStatus::Added,
                old_oid: None,
                new_oid: Some("oid2".to_string()),
                similarity: 0,
                is_binary: false,
            },
        ],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res2 = resolver
        .resolve_commit(&c2_id, &c2_diff, &c2_symbols, &c2_contents)
        .unwrap();
    assert_eq!(res2.touches.len(), 2);

    let orig_touch = res2
        .touches
        .iter()
        .find(|t| t.path == "src/algo.rs")
        .unwrap();
    assert_eq!(orig_touch.lineage_id, orig_lineage);

    let copy_touch = res2
        .touches
        .iter()
        .find(|t| t.path == "src/algo_v2.rs")
        .unwrap();
    assert_ne!(copy_touch.lineage_id, orig_lineage);

    // Derivation DAG records Copy edge
    assert_eq!(res2.derivation_edges.len(), 1);
    let edge = &res2.derivation_edges[0];
    assert_eq!(edge.kind, DerivationKind::Copy);
    assert_eq!(edge.parent_lineage_id, orig_lineage);
    assert_eq!(edge.child_lineage_id, copy_touch.lineage_id);
}

#[test]
fn test_decoy_rejection_allocates_fresh_lineage_m8() {
    let mut resolver = LineageResolver::new();

    // Commit 1: Initial function
    let c1_id = CommitId::new("commit_c1");
    let c1_code = b"fn handle_event() {\n    let a = 1;\n    let b = 2;\n    let c = a + b;\n}\n";
    let mut c1_symbols = HashMap::new();
    c1_symbols.insert(
        PathBuf::from("src/events.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "handle_event".to_string(),
                qualname: "handle_event".to_string(),
                signature: "fn handle_event()".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c1_code.len(),
                start_line: 1,
                end_line: 5,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c1_contents = HashMap::new();
    c1_contents.insert(PathBuf::from("src/events.rs"), c1_code.to_vec());

    let c1_diff = CommitDiff {
        commit_id: c1_id.clone(),
        parent_id: None,
        deltas: vec![FileDelta {
            old_path: None,
            new_path: Some(PathBuf::from("src/events.rs")),
            status: DeltaStatus::Added,
            old_oid: None,
            new_oid: Some("oid1".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res1 = resolver
        .resolve_commit(&c1_id, &c1_diff, &c1_symbols, &c1_contents)
        .unwrap();
    let lineage1 = res1.touches[0].lineage_id;

    // Commit 2: Decoy Delete + Add (M8) - same name, completely unrelated implementation
    let c2_id = CommitId::new("commit_c2");
    let c2_code = b"fn handle_event(ctx: &mut Context, socket: &mut TcpStream) -> Result<(), IoError> {\n    while let Some(msg) = socket.read_msg()? {\n        ctx.dispatch(msg)?;\n    }\n    Ok(())\n}\n";
    let mut c2_symbols = HashMap::new();
    c2_symbols.insert(
        PathBuf::from("src/events.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "handle_event".to_string(),
                qualname: "handle_event".to_string(),
                signature: "fn handle_event(ctx: &mut Context, socket: &mut TcpStream) -> Result<(), IoError>".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c2_code.len(),
                start_line: 1,
                end_line: 6,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c2_contents = HashMap::new();
    c2_contents.insert(PathBuf::from("src/events.rs"), c2_code.to_vec());

    let c2_diff = CommitDiff {
        commit_id: c2_id.clone(),
        parent_id: Some(c1_id),
        deltas: vec![FileDelta {
            old_path: Some(PathBuf::from("src/events.rs")),
            new_path: Some(PathBuf::from("src/events.rs")),
            status: DeltaStatus::Modified,
            old_oid: Some("oid1".to_string()),
            new_oid: Some("oid2".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res2 = resolver
        .resolve_commit(&c2_id, &c2_diff, &c2_symbols, &c2_contents)
        .unwrap();

    // Must be assigned a fresh LineageId and not inherit lineage1
    let lineage2 = res2.touches[0].lineage_id;
    assert_ne!(lineage1, lineage2);
    assert_eq!(res2.resolved_touches[0].match_rule, "new_lineage");
    assert!(res2.derivation_edges.is_empty());
}

#[test]
fn test_uniqueness_invariant_assertion_d11() {
    let mut resolver = LineageResolver::new();

    // Commit 1: Function touch with LineageId(1)
    let c1_id = CommitId::new("commit_c1");
    let c1_code = b"fn test_func() {\n    let val = 42;\n    let res = val * 2;\n}\n";
    let mut c1_symbols = HashMap::new();
    c1_symbols.insert(
        PathBuf::from("src/test.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "test_func".to_string(),
                qualname: "test_func".to_string(),
                signature: "fn test_func()".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c1_code.len(),
                start_line: 1,
                end_line: 4,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c1_contents = HashMap::new();
    c1_contents.insert(PathBuf::from("src/test.rs"), c1_code.to_vec());

    let c1_diff = CommitDiff {
        commit_id: c1_id.clone(),
        parent_id: None,
        deltas: vec![FileDelta {
            old_path: None,
            new_path: Some(PathBuf::from("src/test.rs")),
            status: DeltaStatus::Added,
            old_oid: None,
            new_oid: Some("oid1".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res1 = resolver.resolve_commit(&c1_id, &c1_diff, &c1_symbols, &c1_contents);
    assert!(res1.is_ok());

    // Commit 1 re-executed with modified delta matching test_func to LineageId(1)
    let c1_dup_diff = CommitDiff {
        commit_id: c1_id.clone(),
        parent_id: None,
        deltas: vec![FileDelta {
            old_path: Some(PathBuf::from("src/test.rs")),
            new_path: Some(PathBuf::from("src/test.rs")),
            status: DeltaStatus::Modified,
            old_oid: Some("oid1".to_string()),
            new_oid: Some("oid1".to_string()),
            similarity: 100,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    // Re-resolving same commit with overlapping lineage triggers InvariantViolation
    let res_dup = resolver.resolve_commit(&c1_id, &c1_dup_diff, &c1_symbols, &c1_contents);
    assert!(matches!(res_dup, Err(CoreError::InvariantViolation(_))));
}
