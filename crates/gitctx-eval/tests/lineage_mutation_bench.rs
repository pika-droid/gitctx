//! Integration benchmark tests for synthetic mutation oracle across M1-M10 per LINEAGE-04, D-08, D-13, D-14.

use gitctx_eval::mutator::MutationClass;
use gitctx_eval::oracle::{MutationBenchmarkRunner, ThresholdTuner};

#[test]
fn test_m1_m10_benchmark_accuracy_bars() {
    let runner = MutationBenchmarkRunner::new();
    let report = runner
        .run_benchmark_suite()
        .expect("Benchmark suite execution failed");

    // Print summary tables for test diagnosis
    println!("{}", runner.format_terminal_table(&report));
    println!("{}", runner.format_markdown_table(&report));

    // Verify per-class targets
    for &class in MutationClass::all() {
        let metrics = report
            .per_class
            .get(&class)
            .unwrap_or_else(|| panic!("Missing metrics for {class:?}"));

        match class {
            MutationClass::M1_InPlaceRename => {
                assert!(
                    metrics.f1 >= 0.95,
                    "M1 In-Place Rename F1 ({}) < 0.95 target",
                    metrics.f1
                );
            }
            MutationClass::M2_CrossFileMove => {
                assert!(
                    metrics.f1 >= 0.85,
                    "M2 Cross-File Move F1 ({}) < 0.85 target",
                    metrics.f1
                );
            }
            MutationClass::M3_MoveAndRename => {
                assert!(
                    metrics.f1 >= 0.70,
                    "M3 Move + Rename F1 ({}) < 0.70 target",
                    metrics.f1
                );
            }
            MutationClass::M4_SignatureChange => {
                assert!(
                    metrics.f1 >= 0.95,
                    "M4 Signature Change F1 ({}) < 0.95 target",
                    metrics.f1
                );
            }
            MutationClass::M5_Reformatting => {
                assert!(
                    metrics.f1 >= 0.99,
                    "M5 Reformatting F1 ({}) < 0.99 target",
                    metrics.f1
                );
            }
            MutationClass::M6_LocalVarRename => {
                assert!(
                    metrics.f1 >= 0.98,
                    "M6 Local Var Rename F1 ({}) < 0.98 target",
                    metrics.f1
                );
            }
            MutationClass::M7_FunctionSplit => {
                assert!(
                    metrics.f1 >= 0.90,
                    "M7 Function Split F1 ({}) < 0.90 target",
                    metrics.f1
                );
            }
            MutationClass::M8_DecoyDeleteAdd => {
                let rej_rate = metrics
                    .decoy_rejection_rate
                    .expect("M8 must have decoy rejection rate");
                assert!(
                    rej_rate >= 0.95,
                    "M8 Decoy Rejection Rate ({rej_rate}) < 0.95 target"
                );
            }
            MutationClass::M9_DirectoryMove => {
                assert!(
                    metrics.f1 >= 0.95,
                    "M9 Directory Move F1 ({}) < 0.95 target",
                    metrics.f1
                );
            }
            MutationClass::M10_FunctionCopy => {
                assert!(
                    metrics.f1 >= 0.90,
                    "M10 Function Copy F1 ({}) < 0.90 target",
                    metrics.f1
                );
            }
        }
    }

    // Assert overall metrics
    assert!(
        report.overall_f1 >= 0.90,
        "Overall F1 ({}) < 0.90",
        report.overall_f1
    );
    assert!(
        report.m8_decoy_rejection_rate >= 0.95,
        "Overall M8 decoy rejection rate ({}) < 0.95",
        report.m8_decoy_rejection_rate
    );

    // Assert chain depth metrics
    for depth in [2, 5, 10] {
        let chain_m = report
            .chain_metrics
            .get(&depth)
            .unwrap_or_else(|| panic!("Missing chain metric for depth {depth}"));
        assert!(
            chain_m.recovery_rate >= 0.90,
            "Chain recovery rate at depth D={} ({}) < 0.90",
            depth,
            chain_m.recovery_rate
        );
    }
}

#[test]
fn test_threshold_tuner() {
    let tuner = ThresholdTuner::fast();
    let config = tuner.sweep().expect("Sweep failed");

    assert!(config.tau_same >= 0.50 && config.tau_same <= 0.85);
    assert!(config.tau_cross >= config.tau_same);
    assert!(config.overall_f1 >= 0.90);
    assert!(config.m8_decoy_rejection_rate >= 0.95);

    let temp_dir = tempfile::tempdir().expect("Failed to create tempdir");
    let json_path = temp_dir.path().join("lineage_thresholds.json");

    ThresholdTuner::save_config(&config, &json_path).expect("Save config failed");
    assert!(json_path.exists());

    let loaded = ThresholdTuner::load_config(&json_path).expect("Load config failed");
    assert_eq!(config, loaded);
}
