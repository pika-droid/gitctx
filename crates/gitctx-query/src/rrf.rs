use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default Reciprocal Rank Fusion smoothing parameter per D-14 and INDEX-05.
pub const RRF_DEFAULT_K: f64 = 60.0;

/// Configuration options for Reciprocal Rank Fusion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RrfParams {
    pub k: f64,
    pub exact_weight: f64,
    pub bm25_weight: f64,
    pub dense_weight: f64,
}

impl Default for RrfParams {
    fn default() -> Self {
        Self {
            k: RRF_DEFAULT_K,
            exact_weight: 1.0,
            bm25_weight: 1.0,
            dense_weight: 1.0,
        }
    }
}

impl RrfParams {
    pub fn new(k: f64) -> Self {
        Self {
            k,
            ..Default::default()
        }
    }
}

/// Fused candidate result with aggregate RRF score.
#[derive(Debug, Clone, PartialEq)]
pub struct RrfCandidate {
    pub touch_id: u64,
    pub score: f64,
    pub is_exact_match: bool,
}

/// Merge ranked result lists using Reciprocal Rank Fusion with Rank-0 exact symbol injection per D-13, D-14, and INDEX-05.
///
/// Formula: $\text{RRF\_Score}(d) = \sum \frac{w_m}{k + \text{rank}_m(d)}$
///
/// Exact symbol matches receive Rank-0 priority: $\frac{w_{\text{exact}}}{k + 0}$.
pub fn fuse_rankings(
    exact_matches: &[u64],
    bm25_hits: &[u64],
    dense_hits: &[u64],
    params: &RrfParams,
) -> Vec<RrfCandidate> {
    let mut scores: HashMap<u64, (f64, bool)> = HashMap::new();

    // 1. Inject exact matches at Rank-0
    for &touch_id in exact_matches {
        let entry = scores.entry(touch_id).or_insert((0.0, true));
        entry.0 += params.exact_weight / (params.k + 0.0);
        entry.1 = true;
    }

    // 2. Add BM25 ranked hits (rank = 0, 1, 2, ...)
    for (rank, &touch_id) in bm25_hits.iter().enumerate() {
        let entry = scores.entry(touch_id).or_insert((0.0, false));
        entry.0 += params.bm25_weight / (params.k + rank as f64);
    }

    // 3. Add Dense vector ranked hits (rank = 0, 1, 2, ...)
    for (rank, &touch_id) in dense_hits.iter().enumerate() {
        let entry = scores.entry(touch_id).or_insert((0.0, false));
        entry.0 += params.dense_weight / (params.k + rank as f64);
    }

    // Convert map to sorted candidate vector
    let mut candidates: Vec<RrfCandidate> = scores
        .into_iter()
        .map(|(touch_id, (score, is_exact_match))| RrfCandidate {
            touch_id,
            score,
            is_exact_match,
        })
        .collect();

    // Sort descending by score; break ties deterministically by touch_id ASC
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.touch_id.cmp(&b.touch_id))
    });

    candidates
}
