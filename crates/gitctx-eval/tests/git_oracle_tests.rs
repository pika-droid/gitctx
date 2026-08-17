//! Tests for git log -L heuristic oracle comparator and discrepancy taxonomy per D-15, D-16, LINEAGE-05.

use gitctx_eval::fixtures::mkrepo::RepoBuilder;
use gitctx_eval::oracle::{
    DiscrepancyTaxonomy, FunctionComparisonResult, GitLogOracleComparator, OracleComparisonReport,
};

#[test]
fn test_git_oracle_comparator_mock_classifications() {
    // 1. Full Agreement: same commits in both
    let commits = vec![
        "abcdef1111111111111111111111111111111111".to_string(),
        "abcdef2222222222222222222222222222222222".to_string(),
    ];

    let result = FunctionComparisonResult {
        qualname: "calculate_sum".to_string(),
        path: "math.py".to_string(),
        gitctx_commits: commits.clone(),
        git_log_l_commits: commits.clone(),
        category: DiscrepancyTaxonomy::FullAgreement,
        agreement_rate: 1.0,
        notes: "Exact match".to_string(),
    };
    assert_eq!(result.category, DiscrepancyTaxonomy::FullAgreement);
    assert_eq!(result.agreement_rate, 1.0);

    // 2. gitctx Advantage: gitctx traced across move where git log -L was blind
    let gitctx_commits = vec![
        "abcdef1111111111111111111111111111111111".to_string(),
        "abcdef2222222222222222222222222222222222".to_string(),
        "abcdef3333333333333333333333333333333333".to_string(),
    ];
    let git_log_l_commits = vec!["abcdef3333333333333333333333333333333333".to_string()];

    let advantage_result = FunctionComparisonResult {
        qualname: "process_data".to_string(),
        path: "data_processor.py".to_string(),
        gitctx_commits,
        git_log_l_commits,
        category: DiscrepancyTaxonomy::GitctxAdvantage,
        agreement_rate: 0.333,
        notes: "gitctx traced across cross-file move".to_string(),
    };
    assert_eq!(
        advantage_result.category,
        DiscrepancyTaxonomy::GitctxAdvantage
    );

    // 3. Report formatting
    let report = OracleComparisonReport {
        total_functions_evaluated: 2,
        full_agreement_count: 1,
        gitctx_advantage_count: 1,
        git_log_l_advantage_count: 0,
        ambiguous_count: 0,
        overall_agreement_rate: 0.666,
        oracle_blindness_rate: 0.50,
        results: vec![result, advantage_result],
    };

    let comparator = GitLogOracleComparator;
    let formatted = comparator.format_report(&report);
    assert!(formatted.contains("Full Agreement"));
    assert!(formatted.contains("gitctx Advantage"));
    assert!(formatted.contains("Heuristic Oracle Blindness Rate"));
}

#[test]
fn test_git_oracle_live_repo_execution() {
    let mut builder = RepoBuilder::new().expect("Failed to build test repo");
    builder
        .write_file("calc.py", "def add(a, b):\n    return a + b\n")
        .expect("Write 1 failed");
    builder.commit("Initial add").expect("Commit 1 failed");

    builder
        .write_file(
            "calc.py",
            "def add(a, b):\n    # Optimized\n    return a + b\n",
        )
        .expect("Write 2 failed");
    builder.commit("Update add").expect("Commit 2 failed");

    let sample_funcs = vec![("add".to_string(), "calc.py".to_string(), false)];
    let report = GitLogOracleComparator::evaluate_repository(builder.path(), &sample_funcs)
        .expect("Evaluation failed");

    assert_eq!(report.total_functions_evaluated, 1);
    assert!(report.overall_agreement_rate >= 0.0);
}
