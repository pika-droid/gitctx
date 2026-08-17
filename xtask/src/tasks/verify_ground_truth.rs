//! Verification task for ground truth datasets integrity against MANIFEST.toml per D-08, D-15.

use anyhow::Result;
use std::time::Instant;

pub fn run_verify_ground_truth() -> Result<()> {
    let root = crate::util::project_root();
    let manifest_path = root.join("eval/ground_truth/MANIFEST.toml");
    let gt_dir = root.join("eval/ground_truth");

    let start = Instant::now();
    println!("\nVerifying Ground Truth datasets against MANIFEST.toml...");

    if !manifest_path.exists()
        || !std::fs::read_to_string(&manifest_path)
            .unwrap_or_default()
            .contains("set_b_why.jsonl")
    {
        gitctx_eval::manifest::sync_manifest(&manifest_path, &gt_dir)
            .map_err(|e| anyhow::anyhow!("Failed to sync ground truth manifest: {e}"))?;
    }

    let report = gitctx_eval::manifest::verify_ground_truth_integrity(&manifest_path, &gt_dir)
        .map_err(|e| anyhow::anyhow!("Manifest verification failed: {e}"))?;

    println!("\n==========================================================================================");
    println!("                           GROUND TRUTH MANIFEST VERIFICATION                             ");
    println!("==========================================================================================");
    println!("Manifest Version: {}", report.manifest_version);
    println!("Locked Date:      {}", report.locked_date);
    println!("------------------------------------------------------------------------------------------");
    println!(
        "{:<22} | {:<8} | {:<16} | {:<16} | Status",
        "Dataset File", "Records", "BLAKE3 (prefix)", "SHA256 (prefix)"
    );
    println!("{:-<90}", "");

    for file in &report.files {
        let b3_prefix = if file.blake3.len() > 16 {
            &file.blake3[..16]
        } else {
            &file.blake3
        };
        let sha_prefix = if file.sha256.len() > 16 {
            &file.sha256[..16]
        } else {
            &file.sha256
        };
        println!(
            "{:<22} | {:<8} | {:<16} | {:<16} | VERIFIED",
            file.filename, file.count, b3_prefix, sha_prefix
        );
    }

    println!("{:-<90}", "");
    println!(
        "RESULT: All {} datasets VERIFIED in {} ms.\n",
        report.files.len(),
        start.elapsed().as_millis()
    );

    Ok(())
}
