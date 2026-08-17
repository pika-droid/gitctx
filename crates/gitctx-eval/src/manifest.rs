//! Cryptographic Ground Truth Manifest verification and integrity checking per D-08, D-15.

use gitctx_core::{CoreError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Ground truth manifest schema mapping files to their cryptographic hashes and record counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundTruthManifest {
    pub version: u32,
    pub locked_date: String,
    pub files: BTreeMap<String, ManifestFileEntry>,
}

/// Metadata entry for a single ground truth dataset file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFileEntry {
    pub count: usize,
    pub blake3: String,
    pub sha256: String,
}

/// File verification result entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedFileReport {
    pub filename: String,
    pub count: usize,
    pub blake3: String,
    pub sha256: String,
    pub verified: bool,
}

/// Overall ground truth verification report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestVerificationReport {
    pub manifest_version: u32,
    pub locked_date: String,
    pub files: Vec<VerifiedFileReport>,
    pub all_passed: bool,
}

/// Verifies all datasets listed in `MANIFEST.toml` against their byte contents on disk.
///
/// Computes BLAKE3 and SHA256 hashes and line counts for every file,
/// asserting strict equality against the manifest.
///
/// # Errors
/// Fails with `CoreError::GroundTruthCorrupted` if any hash or count mismatches,
/// or `CoreError::Io` / `CoreError::ParseError` on file access/format errors.
pub fn verify_ground_truth_integrity(
    manifest_path: &Path,
    ground_truth_dir: &Path,
) -> Result<ManifestVerificationReport> {
    let manifest_content =
        fs::read_to_string(manifest_path).map_err(|e| CoreError::RepositoryOpen {
            path: manifest_path.to_path_buf(),
            message: format!("Failed to read manifest file: {e}"),
        })?;

    let manifest: GroundTruthManifest = toml::from_str(&manifest_content).map_err(|e| {
        CoreError::ParseError(format!("Failed to parse ground truth MANIFEST.toml: {e}"))
    })?;

    let mut reports = Vec::new();
    let all_passed = true;

    for (filename, expected) in &manifest.files {
        let file_path = ground_truth_dir.join(filename);
        if !file_path.exists() {
            return Err(CoreError::GroundTruthCorrupted {
                file: filename.clone(),
                message: format!("Ground truth file not found at {}", file_path.display()),
            });
        }

        let raw_bytes: Vec<u8> = fs::read(&file_path)?.into_iter().filter(|&b| b != b'\r').collect();
        let content_str = String::from_utf8_lossy(&raw_bytes);
        let actual_count = content_str
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .count();

        let actual_blake3 = blake3::hash(&raw_bytes).to_hex().to_string();

        let mut hasher = Sha256::new();
        hasher.update(&raw_bytes);
        let actual_sha256 = format!("{:x}", hasher.finalize());

        if actual_count != expected.count {
            return Err(CoreError::GroundTruthCorrupted {
                file: filename.clone(),
                message: format!(
                    "Record count mismatch: expected {}, got {}",
                    expected.count, actual_count
                ),
            });
        }

        if actual_blake3 != expected.blake3 {
            return Err(CoreError::GroundTruthCorrupted {
                file: filename.clone(),
                message: format!(
                    "BLAKE3 checksum mismatch: expected {}, got {}",
                    expected.blake3, actual_blake3
                ),
            });
        }

        if actual_sha256 != expected.sha256 {
            return Err(CoreError::GroundTruthCorrupted {
                file: filename.clone(),
                message: format!(
                    "SHA256 checksum mismatch: expected {}, got {}",
                    expected.sha256, actual_sha256
                ),
            });
        }

        reports.push(VerifiedFileReport {
            filename: filename.clone(),
            count: actual_count,
            blake3: actual_blake3,
            sha256: actual_sha256,
            verified: true,
        });
    }

    Ok(ManifestVerificationReport {
        manifest_version: manifest.version,
        locked_date: manifest.locked_date,
        files: reports,
        all_passed,
    })
}

/// Helper function to compute hash entry for a dataset file.
pub fn compute_file_entry(file_path: &Path) -> Result<ManifestFileEntry> {
    let raw_bytes: Vec<u8> = fs::read(file_path)?.into_iter().filter(|&b| b != b'\r').collect();
    let content_str = String::from_utf8_lossy(&raw_bytes);
    let count = content_str
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .count();

    let blake3_hex = blake3::hash(&raw_bytes).to_hex().to_string();

    let mut hasher = Sha256::new();
    hasher.update(&raw_bytes);
    let sha256_hex = format!("{:x}", hasher.finalize());

    Ok(ManifestFileEntry {
        count,
        blake3: blake3_hex,
        sha256: sha256_hex,
    })
}

/// Computes hashes and line counts for all ground truth datasets and writes `MANIFEST.toml`.
pub fn sync_manifest(manifest_path: &Path, ground_truth_dir: &Path) -> Result<GroundTruthManifest> {
    let files = [
        "revert_pairs.jsonl",
        "set_a_issues.jsonl",
        "set_b_why.jsonl",
    ];
    let mut file_entries = BTreeMap::new();

    for filename in files {
        let file_path = ground_truth_dir.join(filename);
        if !file_path.exists() {
            return Err(CoreError::GroundTruthCorrupted {
                file: filename.to_string(),
                message: format!("Ground truth file not found at {}", file_path.display()),
            });
        }
        let entry = compute_file_entry(&file_path)?;
        file_entries.insert(filename.to_string(), entry);
    }

    let manifest = GroundTruthManifest {
        version: 1,
        locked_date: "2026-08-16T21:00:00Z".to_string(),
        files: file_entries,
    };

    let mut lines = vec![
        "# Cryptographic Ground Truth Manifest".to_string(),
        "# Locks dataset integrity before Phase 3 Python ablation (D-08, D-15)".to_string(),
        "version = 1".to_string(),
        format!("locked_date = \"{}\"", manifest.locked_date),
        String::new(),
    ];

    for (filename, entry) in &manifest.files {
        lines.push(format!("[files.\"{filename}\"]"));
        lines.push(format!("count = {}", entry.count));
        lines.push(format!("blake3 = \"{}\"", entry.blake3));
        lines.push(format!("sha256 = \"{}\"", entry.sha256));
        lines.push(String::new());
    }

    let content = lines.join("\n");
    fs::write(manifest_path, content)?;

    Ok(manifest)
}
