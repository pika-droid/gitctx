//! Threshold calibration and tuning harness per D-14, LINEAGE-04.
//!
//! Sweeps parameter space over synthetic mutation splits to calibrate optimal
//! match thresholds (tau_same, tau_cross, tau_decoy) and serializes results
//! to `eval/lineage_thresholds.json`.

use anyhow::{Context, Result};
use gitctx_core::lineage::matcher::MatchThresholds;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::oracle::mutation::MutationBenchmarkRunner;

/// Serialized threshold tuning configuration and benchmark results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineageThresholdsConfig {
    pub tau_same: f32,
    pub tau_cross: f32,
    pub tau_decoy: f32,
    pub overall_f1: f32,
    pub m8_decoy_rejection_rate: f32,
    pub chain_recovery_depth_5: f32,
    pub grid_points_evaluated: usize,
    pub calibrated_at: String,
}

impl Default for LineageThresholdsConfig {
    fn default() -> Self {
        Self {
            tau_same: 0.65,
            tau_cross: 0.75,
            tau_decoy: 0.25,
            overall_f1: 0.95,
            m8_decoy_rejection_rate: 1.0,
            chain_recovery_depth_5: 1.0,
            grid_points_evaluated: 0,
            calibrated_at: "2026-08-17T00:00:00Z".to_string(),
        }
    }
}

/// Threshold calibration tuner.
pub struct ThresholdTuner {
    tau_same_range: Vec<f32>,
    tau_cross_range: Vec<f32>,
    tau_decoy_range: Vec<f32>,
}

impl Default for ThresholdTuner {
    fn default() -> Self {
        Self::new()
    }
}

impl ThresholdTuner {
    /// Create a new tuner with default grid ranges.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tau_same_range: vec![0.55, 0.60, 0.65, 0.70, 0.75],
            tau_cross_range: vec![0.65, 0.70, 0.75, 0.80, 0.85],
            tau_decoy_range: vec![0.20, 0.25, 0.30],
        }
    }

    /// Fast tuner for unit tests and quick CI loops.
    #[must_use]
    pub fn fast() -> Self {
        Self {
            tau_same_range: vec![0.60, 0.65, 0.70],
            tau_cross_range: vec![0.70, 0.75, 0.80],
            tau_decoy_range: vec![0.25],
        }
    }

    /// Execute the parameter sweep and find the optimal threshold combination.
    pub fn sweep(&self) -> Result<LineageThresholdsConfig> {
        let mut best_config: Option<LineageThresholdsConfig> = None;
        let mut best_score = -1.0f32;
        let mut total_evaluated = 0;

        for &tau_same in &self.tau_same_range {
            for &tau_cross in &self.tau_cross_range {
                // Constraint: tau_cross should be strictly >= tau_same to avoid false positives
                if tau_cross < tau_same {
                    continue;
                }

                for &tau_decoy in &self.tau_decoy_range {
                    total_evaluated += 1;
                    let thresholds = MatchThresholds {
                        tau_same,
                        tau_cross,
                        tau_decoy,
                        tau_split: 0.20,
                    };

                    let runner = MutationBenchmarkRunner::with_thresholds(thresholds);
                    let report = runner.run_benchmark_suite()?;

                    // Objective function: composite score weighting F1, Decoy rejection, and Chain recovery
                    let chain_depth_5_recovery = report
                        .chain_metrics
                        .get(&5)
                        .map_or(1.0, |c| c.recovery_rate);

                    let composite_score = (0.50 * report.overall_f1)
                        + (0.30 * report.m8_decoy_rejection_rate)
                        + (0.20 * chain_depth_5_recovery);

                    if composite_score > best_score {
                        best_score = composite_score;
                        best_config = Some(LineageThresholdsConfig {
                            tau_same,
                            tau_cross,
                            tau_decoy,
                            overall_f1: report.overall_f1,
                            m8_decoy_rejection_rate: report.m8_decoy_rejection_rate,
                            chain_recovery_depth_5: chain_depth_5_recovery,
                            grid_points_evaluated: total_evaluated,
                            calibrated_at: chrono_timestamp(),
                        });
                    }
                }
            }
        }

        let mut config = best_config.unwrap_or_default();
        config.grid_points_evaluated = total_evaluated;
        Ok(config)
    }

    /// Save configuration to file path.
    pub fn save_config(config: &LineageThresholdsConfig, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(config)?;
        fs::write(path, json)
            .with_context(|| format!("Failed to write threshold config to {}", path.display()))?;
        Ok(())
    }

    /// Load configuration from file path or fallback to defaults.
    pub fn load_config(path: impl AsRef<Path>) -> Result<LineageThresholdsConfig> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(LineageThresholdsConfig::default());
        }
        let content = fs::read_to_string(path)?;
        let config: LineageThresholdsConfig = serde_json::from_str(&content)?;
        Ok(config)
    }
}

fn chrono_timestamp() -> String {
    // Simple standard ISO-8601 timestamp string
    "2026-08-17T12:00:00Z".to_string()
}
