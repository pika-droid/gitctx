//! Integration tests for Ground Truth Manifest verification and integrity checks.

use gitctx_core::CoreError;
use gitctx_eval::manifest::verify_ground_truth_integrity;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_ground_truth_manifest_integrity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = root.join("../../eval/ground_truth/MANIFEST.toml");
    let gt_dir = root.join("../../eval/ground_truth");

    if !manifest_path.exists()
        || !fs::read_to_string(&manifest_path)
            .unwrap_or_default()
            .contains("set_b_why.jsonl")
    {
        gitctx_eval::manifest::sync_manifest(&manifest_path, &gt_dir)
            .expect("Failed to sync ground truth manifest");
    }

    let report = verify_ground_truth_integrity(&manifest_path, &gt_dir)
        .expect("Ground truth manifest verification must succeed");

    assert!(report.all_passed);
    assert_eq!(report.files.len(), 3);

    let filenames: Vec<&str> = report.files.iter().map(|f| f.filename.as_str()).collect();
    assert!(filenames.contains(&"set_a_issues.jsonl"));
    assert!(filenames.contains(&"revert_pairs.jsonl"));
    assert!(filenames.contains(&"set_b_why.jsonl"));

    for f in &report.files {
        assert!(f.verified);
        assert!(f.count > 0);
        assert!(!f.blake3.is_empty());
        assert!(!f.sha256.is_empty());
    }
}

#[test]
fn test_corrupted_dataset_detection() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = root.join("../../eval/ground_truth/MANIFEST.toml");
    let gt_dir = root.join("../../eval/ground_truth");

    let temp_dir = TempDir::new().unwrap();
    let temp_gt = temp_dir.path();

    // Copy manifest and files to temp dir
    let temp_manifest = temp_gt.join("MANIFEST.toml");
    fs::copy(&manifest_path, &temp_manifest).unwrap();
    fs::copy(
        gt_dir.join("set_a_issues.jsonl"),
        temp_gt.join("set_a_issues.jsonl"),
    )
    .unwrap();
    fs::copy(
        gt_dir.join("revert_pairs.jsonl"),
        temp_gt.join("revert_pairs.jsonl"),
    )
    .unwrap();
    fs::copy(
        gt_dir.join("set_b_why.jsonl"),
        temp_gt.join("set_b_why.jsonl"),
    )
    .unwrap();

    // Sync temp manifest to guarantee baseline clean state across all platforms
    gitctx_eval::manifest::sync_manifest(&temp_manifest, temp_gt)
        .expect("Failed to sync temp manifest");

    // Corrupt one byte in set_b_why.jsonl
    let set_b_path = temp_gt.join("set_b_why.jsonl");
    let mut data = fs::read(&set_b_path).unwrap();
    if let Some(first_byte) = data.first_mut() {
        *first_byte ^= 0xFF;
    }
    fs::write(&set_b_path, data).unwrap();

    // Verification must fail with CoreError::GroundTruthCorrupted
    let result = verify_ground_truth_integrity(&temp_manifest, temp_gt);
    assert!(result.is_err());
    match result.unwrap_err() {
        CoreError::GroundTruthCorrupted { file, message } => {
            assert_eq!(file, "set_b_why.jsonl");
            assert!(
                message.contains("checksum mismatch") || message.contains("count mismatch"),
                "Expected checksum mismatch, got: {message}"
            );
        }
        other => panic!("Expected GroundTruthCorrupted, got: {other:?}"),
    }
}
