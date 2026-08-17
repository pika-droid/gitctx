use gitctx_core::lineage::matcher::{
    BipartiteMatcher, CandidateGenerator, CandidateMatch, MatchThresholds, MatchTier,
    SymbolDescriptor,
};
use gitctx_core::lineage::minhash::MinHashFingerprint;
use gitctx_core::model::{LineageId, ParseStatus};
use gitctx_core::parser::LanguageId;
use std::collections::HashMap;
use std::path::PathBuf;

fn make_symbol(
    lineage_id: Option<u64>,
    path: &str,
    qualname: &str,
    lang: LanguageId,
    tokens: &[&str],
    signature: &str,
) -> SymbolDescriptor {
    let token_vec: Vec<String> = tokens.iter().map(|s| s.to_string()).collect();
    let (fingerprint, is_short) = MinHashFingerprint::from_tokens(&token_vec);
    SymbolDescriptor {
        lineage_id: lineage_id.map(LineageId::new),
        path: path.to_string(),
        qualname: qualname.to_string(),
        language: lang,
        fingerprint,
        token_count: token_vec.len(),
        is_short,
        signature: signature.to_string(),
        docstring: None,
        start_line: 1,
        end_line: 10,
        parse_status: ParseStatus::Ok,
    }
}

#[test]
fn test_bipartite_deterministic_tie_breaking() {
    let cand1 = CandidateMatch {
        parent_index: 0,
        child_index: 0,
        tier: MatchTier::ExactKey,
        score: 1.0,
        old_path: "src/a.rs".to_string(),
        old_qualname: "foo".to_string(),
        new_path: "src/a.rs".to_string(),
        new_qualname: "foo".to_string(),
    };
    let cand2 = CandidateMatch {
        parent_index: 1,
        child_index: 1,
        tier: MatchTier::ExactKey,
        score: 1.0,
        old_path: "src/b.rs".to_string(),
        old_qualname: "bar".to_string(),
        new_path: "src/b.rs".to_string(),
        new_qualname: "bar".to_string(),
    };

    assert_eq!(
        cand1.cmp_deterministic(&cand2),
        std::cmp::Ordering::Less // cand1 ("src/a.rs") before cand2 ("src/b.rs")
    );

    let solution = BipartiteMatcher::solve(vec![cand2.clone(), cand1.clone()], 2, 2);
    assert_eq!(solution.matches.len(), 2);
    assert_eq!(solution.matches[0].old_path, "src/a.rs");
    assert_eq!(solution.matches[1].old_path, "src/b.rs");
    assert!(solution.unmatched_parents.is_empty());
    assert!(solution.unmatched_children.is_empty());
}

#[test]
fn test_bipartite_greedy_conflict_elimination() {
    // Two competing matches for child 0: parent 0 (score 0.95) and parent 1 (score 0.70)
    let cand1 = CandidateMatch {
        parent_index: 0,
        child_index: 0,
        tier: MatchTier::SameFileMinHash,
        score: 0.95,
        old_path: "src/a.rs".to_string(),
        old_qualname: "func_old".to_string(),
        new_path: "src/a.rs".to_string(),
        new_qualname: "func_new".to_string(),
    };
    let cand2 = CandidateMatch {
        parent_index: 1,
        child_index: 0,
        tier: MatchTier::SameFileMinHash,
        score: 0.70,
        old_path: "src/a.rs".to_string(),
        old_qualname: "other_old".to_string(),
        new_path: "src/a.rs".to_string(),
        new_qualname: "func_new".to_string(),
    };

    let solution = BipartiteMatcher::solve(vec![cand2, cand1], 2, 1);
    assert_eq!(solution.matches.len(), 1);
    assert_eq!(solution.matches[0].parent_index, 0);
    assert_eq!(solution.unmatched_parents, vec![1]);
    assert!(solution.unmatched_children.is_empty());
}

#[test]
fn test_candidate_generation_tiers() {
    let tokens_a = [
        "<kw:fn>", "var_0", "<op:(>", "var_1", "<op::>", "var_2", "<op:)>",
        "<kw:return>", "var_1", "<op:+>", "<num>", "<op:;>",
    ];
    let tokens_b = [
        "<kw:fn>", "var_0", "<op:(>", "var_1", "<op::>", "var_2", "<op:)>",
        "<kw:return>", "var_1", "<op:+>", "<num>", "<op:;>",
    ];

    let parent = make_symbol(
        Some(1),
        "src/lib.rs",
        "calculate_sum",
        LanguageId::Rust,
        &tokens_a,
        "fn calculate_sum(x: i32) -> i32",
    );
    let child_exact = make_symbol(
        None,
        "src/lib.rs",
        "calculate_sum",
        LanguageId::Rust,
        &tokens_b,
        "fn calculate_sum(x: i32) -> i32",
    );

    let renames = HashMap::new();
    let thresholds = MatchThresholds::default();

    let candidates = CandidateGenerator::generate_candidates(
        &[parent.clone()],
        &[child_exact],
        &renames,
        &thresholds,
    );

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].tier, MatchTier::ExactKey);
    assert_eq!(candidates[0].score, 1.0);
}

#[test]
fn test_decoy_rejection_m8() {
    let tokens_old = [
        "<kw:fn>", "var_0", "<op:(>", "<op:)>", "<kw:return>", "<num>", "<op:;>",
        "<op:;>", "<op:;>", "<op:;>", "<op:;>",
    ];
    let tokens_unrelated = [
        "<kw:fn>", "var_a", "<kw:while>", "var_b", "<kw:for>", "var_c", "<kw:try>",
        "var_d", "<kw:match>", "var_e", "<kw:loop>", "var_f",
    ];

    let parent = make_symbol(
        Some(1),
        "src/service.rs",
        "process_item",
        LanguageId::Rust,
        &tokens_old,
        "fn process_item() -> u32",
    );
    let child_decoy = make_symbol(
        None,
        "src/service.rs",
        "process_item",
        LanguageId::Rust,
        &tokens_unrelated,
        "fn process_item(stream: &mut Stream) -> Result<(), Error>",
    );

    let renames = HashMap::new();
    let thresholds = MatchThresholds::default();

    let candidates =
        CandidateGenerator::generate_candidates(&[parent], &[child_decoy], &renames, &thresholds);

    // Decoy rejection: similarity is near 0.0 (< 0.25) and AST changed -> no candidate produced
    assert!(candidates.is_empty());
}

#[test]
fn test_short_function_length_gating() {
    // Functions with < 10 tokens (short functions) must not match via MinHash
    let short_tokens_1 = ["<kw:fn>", "var_0", "<kw:return>", "var_1"];
    let short_tokens_2 = ["<kw:fn>", "var_2", "<kw:return>", "var_3"];

    let parent = make_symbol(
        Some(1),
        "src/models.rs",
        "get_id",
        LanguageId::Rust,
        &short_tokens_1,
        "fn get_id(&self) -> u32",
    );
    let child_renamed = make_symbol(
        None,
        "src/models.rs",
        "id",
        LanguageId::Rust,
        &short_tokens_2,
        "fn id(&self) -> u32",
    );

    let renames = HashMap::new();
    let thresholds = MatchThresholds::default();

    let candidates = CandidateGenerator::generate_candidates(
        &[parent],
        &[child_renamed],
        &renames,
        &thresholds,
    );

    // Short function should NOT match via Tier 2 SameFileMinHash
    assert!(candidates.is_empty());
}

#[test]
fn test_cross_file_and_language_isolation() {
    let tokens = [
        "<kw:fn>", "var_0", "<op:(>", "var_1", "<op::>", "var_2", "<op:)>",
        "<kw:return>", "var_1", "<op:+>", "<num>", "<op:;>",
    ];

    let parent_rust = make_symbol(
        Some(1),
        "src/old_path.rs",
        "compute",
        LanguageId::Rust,
        &tokens,
        "fn compute(x: i32) -> i32",
    );
    let child_rust = make_symbol(
        None,
        "src/new_path.rs",
        "compute_moved",
        LanguageId::Rust,
        &tokens,
        "fn compute_moved(x: i32) -> i32",
    );
    let child_python = make_symbol(
        None,
        "src/new_path.py",
        "compute_moved",
        LanguageId::Python,
        &tokens,
        "def compute_moved(x):",
    );

    let renames = HashMap::new();
    let thresholds = MatchThresholds::default();

    // Cross-file with same language -> Tier 4 match
    let candidates_rust = CandidateGenerator::generate_candidates(
        &[parent_rust.clone()],
        &[child_rust],
        &renames,
        &thresholds,
    );
    assert_eq!(candidates_rust.len(), 1);
    assert_eq!(candidates_rust[0].tier, MatchTier::CrossFileMinHash);

    // Cross-file with different language -> Skipped
    let candidates_py = CandidateGenerator::generate_candidates(
        &[parent_rust],
        &[child_python],
        &renames,
        &thresholds,
    );
    assert!(candidates_py.is_empty());
}

#[test]
fn test_git_rename_mapping() {
    let tokens = [
        "<kw:fn>", "var_0", "<op:(>", "var_1", "<op::>", "var_2", "<op:)>",
        "<kw:return>", "var_1", "<op:+>", "<num>", "<op:;>",
    ];

    let parent = make_symbol(
        Some(1),
        "src/old_file.rs",
        "process",
        LanguageId::Rust,
        &tokens,
        "fn process(x: i32) -> i32",
    );
    let child = make_symbol(
        None,
        "src/new_file.rs",
        "process",
        LanguageId::Rust,
        &tokens,
        "fn process(x: i32) -> i32",
    );

    let mut renames = HashMap::new();
    renames.insert(
        PathBuf::from("src/old_file.rs"),
        PathBuf::from("src/new_file.rs"),
    );
    let thresholds = MatchThresholds::default();

    let candidates =
        CandidateGenerator::generate_candidates(&[parent], &[child], &renames, &thresholds);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].tier, MatchTier::GitRenameExact);
    assert_eq!(candidates[0].score, 0.95);
}

#[test]
fn test_multi_commit_lineage_continuity() {
    use gitctx_core::git::{CommitDiff, DeltaStatus, FileDelta};
    use gitctx_core::lineage::LineageResolver;
    use gitctx_core::model::CommitId;
    use gitctx_core::parser::{ParsedFileSymbols, RawFunctionSymbol};

    let mut resolver = LineageResolver::new();

    // Commit 1: Add src/service.rs with calculate_tax
    let c1_id = CommitId::new("commit_1");
    let c1_code = b"fn calculate_tax(amount: f64, rate: f64) -> f64 {\n    let base = amount * rate;\n    let surcharge = base * 0.05;\n    base + surcharge\n}\n";
    let mut c1_symbols = HashMap::new();
    c1_symbols.insert(
        PathBuf::from("src/service.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "calculate_tax".to_string(),
                qualname: "calculate_tax".to_string(),
                signature: "fn calculate_tax(amount: f64, rate: f64) -> f64".to_string(),
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
    c1_contents.insert(PathBuf::from("src/service.rs"), c1_code.to_vec());

    let c1_diff = CommitDiff {
        commit_id: c1_id.clone(),
        parent_id: None,
        deltas: vec![FileDelta {
            old_path: None,
            new_path: Some(PathBuf::from("src/service.rs")),
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
    assert_eq!(res1.touches[0].lineage_id.as_u64(), 1);
    assert_eq!(res1.resolved_touches[0].match_rule, "new_lineage");

    // Commit 2: Edit calculate_tax in-place (Tier 1 ExactKey)
    let c2_id = CommitId::new("commit_2");
    let c2_code = b"fn calculate_tax(amount: f64, rate: f64) -> f64 {\n    let base = amount * rate;\n    let surcharge = base * 0.08;\n    base + surcharge\n}\n";
    let mut c2_symbols = HashMap::new();
    c2_symbols.insert(
        PathBuf::from("src/service.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "calculate_tax".to_string(),
                qualname: "calculate_tax".to_string(),
                signature: "fn calculate_tax(amount: f64, rate: f64) -> f64".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c2_code.len(),
                start_line: 1,
                end_line: 5,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c2_contents = HashMap::new();
    c2_contents.insert(PathBuf::from("src/service.rs"), c2_code.to_vec());

    let c2_diff = CommitDiff {
        commit_id: c2_id.clone(),
        parent_id: Some(c1_id),
        deltas: vec![FileDelta {
            old_path: Some(PathBuf::from("src/service.rs")),
            new_path: Some(PathBuf::from("src/service.rs")),
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
    assert_eq!(res2.touches[0].lineage_id.as_u64(), 1);
    assert_eq!(res2.resolved_touches[0].match_rule, "exact_key");

    // Commit 3: In-place Rename to compute_tax (Tier 2 SameFileMinHash)
    let c3_id = CommitId::new("commit_3");
    let c3_code = b"fn compute_tax(amount: f64, rate: f64) -> f64 {\n    let base = amount * rate;\n    let surcharge = base * 0.08;\n    base + surcharge\n}\n";
    let mut c3_symbols = HashMap::new();
    c3_symbols.insert(
        PathBuf::from("src/service.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "compute_tax".to_string(),
                qualname: "compute_tax".to_string(),
                signature: "fn compute_tax(amount: f64, rate: f64) -> f64".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c3_code.len(),
                start_line: 1,
                end_line: 5,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c3_contents = HashMap::new();
    c3_contents.insert(PathBuf::from("src/service.rs"), c3_code.to_vec());

    let c3_diff = CommitDiff {
        commit_id: c3_id.clone(),
        parent_id: Some(c2_id),
        deltas: vec![FileDelta {
            old_path: Some(PathBuf::from("src/service.rs")),
            new_path: Some(PathBuf::from("src/service.rs")),
            status: DeltaStatus::Modified,
            old_oid: Some("oid2".to_string()),
            new_oid: Some("oid3".to_string()),
            similarity: 0,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res3 = resolver
        .resolve_commit(&c3_id, &c3_diff, &c3_symbols, &c3_contents)
        .unwrap();
    assert_eq!(res3.touches.len(), 1);
    assert_eq!(res3.touches[0].lineage_id.as_u64(), 1);
    assert_eq!(
        res3.resolved_touches[0].match_rule,
        "same_file_fingerprint"
    );

    // Commit 4: Cross-file Move to src/calculator.rs (Tier 4 CrossFileMinHash)
    let c4_id = CommitId::new("commit_4");
    let c4_code_calc = b"fn compute_tax(amount: f64, rate: f64) -> f64 {\n    let base = amount * rate;\n    let surcharge = base * 0.08;\n    base + surcharge\n}\n";
    let c4_code_serv = b"// empty service\n";
    let mut c4_symbols = HashMap::new();
    c4_symbols.insert(
        PathBuf::from("src/service.rs"),
        ParsedFileSymbols {
            symbols: vec![],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    c4_symbols.insert(
        PathBuf::from("src/calculator.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "compute_tax".to_string(),
                qualname: "compute_tax".to_string(),
                signature: "fn compute_tax(amount: f64, rate: f64) -> f64".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c4_code_calc.len(),
                start_line: 1,
                end_line: 5,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c4_contents = HashMap::new();
    c4_contents.insert(PathBuf::from("src/service.rs"), c4_code_serv.to_vec());
    c4_contents.insert(
        PathBuf::from("src/calculator.rs"),
        c4_code_calc.to_vec(),
    );

    let c4_diff = CommitDiff {
        commit_id: c4_id.clone(),
        parent_id: Some(c3_id),
        deltas: vec![
            FileDelta {
                old_path: Some(PathBuf::from("src/service.rs")),
                new_path: Some(PathBuf::from("src/service.rs")),
                status: DeltaStatus::Modified,
                old_oid: Some("oid3".to_string()),
                new_oid: Some("oid4a".to_string()),
                similarity: 0,
                is_binary: false,
            },
            FileDelta {
                old_path: None,
                new_path: Some(PathBuf::from("src/calculator.rs")),
                status: DeltaStatus::Added,
                old_oid: None,
                new_oid: Some("oid4b".to_string()),
                similarity: 0,
                is_binary: false,
            },
        ],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res4 = resolver
        .resolve_commit(&c4_id, &c4_diff, &c4_symbols, &c4_contents)
        .unwrap();
    assert_eq!(res4.touches.len(), 1);
    assert_eq!(res4.touches[0].lineage_id.as_u64(), 1);
    assert_eq!(
        res4.resolved_touches[0].match_rule,
        "cross_file_fingerprint"
    );

    // Commit 5: Git Rename File src/calculator.rs -> src/tax/engine.rs (Tier 3 GitRenameExact)
    let c5_id = CommitId::new("commit_5");
    let mut c5_symbols = HashMap::new();
    c5_symbols.insert(
        PathBuf::from("src/tax/engine.rs"),
        ParsedFileSymbols {
            symbols: vec![RawFunctionSymbol {
                name: "compute_tax".to_string(),
                qualname: "compute_tax".to_string(),
                signature: "fn compute_tax(amount: f64, rate: f64) -> f64".to_string(),
                docstring: None,
                start_byte: 0,
                end_byte: c4_code_calc.len(),
                start_line: 1,
                end_line: 5,
                is_method: false,
            }],
            status: ParseStatus::Ok,
            error_byte_ranges: vec![],
        },
    );
    let mut c5_contents = HashMap::new();
    c5_contents.insert(
        PathBuf::from("src/tax/engine.rs"),
        c4_code_calc.to_vec(),
    );

    let c5_diff = CommitDiff {
        commit_id: c5_id.clone(),
        parent_id: Some(c4_id),
        deltas: vec![FileDelta {
            old_path: Some(PathBuf::from("src/calculator.rs")),
            new_path: Some(PathBuf::from("src/tax/engine.rs")),
            status: DeltaStatus::Renamed,
            old_oid: Some("oid4b".to_string()),
            new_oid: Some("oid5".to_string()),
            similarity: 100,
            is_binary: false,
        }],
        hunks_by_path: HashMap::new(),
        raw_diff: String::new(),
    };

    let res5 = resolver
        .resolve_commit(&c5_id, &c5_diff, &c5_symbols, &c5_contents)
        .unwrap();
    assert_eq!(res5.touches.len(), 1);
    assert_eq!(res5.touches[0].lineage_id.as_u64(), 1);
    assert_eq!(res5.resolved_touches[0].match_rule, "git_rename_exact");

}

