use crate::vector::mmap::MmapVectorStorage;
use rayon::prelude::*;

/// In-place unit L2 normalization ($\hat{\mathbf{v}} = \mathbf{v} / \|\mathbf{v}\|_2$) per D-11.
pub fn normalize_l2(vec: &mut [f32]) {
    let sum_sq: f32 = vec.iter().map(|v| v * v).sum();
    let norm = sum_sq.sqrt();
    if norm > 1e-12 {
        let inv_norm = 1.0 / norm;
        for val in vec.iter_mut() {
            *val *= inv_norm;
        }
    } else {
        vec.fill(0.0);
    }
}

/// Compute and return a unit L2-normalized copy of `vec`.
pub fn normalized_l2(vec: &[f32]) -> Vec<f32> {
    let mut out = vec.to_vec();
    normalize_l2(&mut out);
    out
}

/// Compute SIMD-friendly dot product between two equal-length f32 slices.
/// Uses 4-way loop unrolling to facilitate CPU auto-vectorization (AVX2/NEON/SSE).
#[inline]
pub fn dot_product_simd(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Vectors must have identical dimensions");

    let mut sum0 = 0.0f32;
    let mut sum1 = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut sum3 = 0.0f32;

    let chunks_a = a.chunks_exact(4);
    let chunks_b = b.chunks_exact(4);
    let rem_a = chunks_a.remainder();
    let rem_b = chunks_b.remainder();

    for (ca, cb) in chunks_a.zip(chunks_b) {
        sum0 += ca[0] * cb[0];
        sum1 += ca[1] * cb[1];
        sum2 += ca[2] * cb[2];
        sum3 += ca[3] * cb[3];
    }

    let mut total = sum0 + sum1 + sum2 + sum3;
    for (x, y) in rem_a.iter().zip(rem_b.iter()) {
        total += x * y;
    }
    total
}

/// Execute parallel SIMD cosine search over memory-mapped vectors using Rayon per D-11 and INDEX-04.
///
/// Since vectors in `storage` are unit L2-normalized at write time, cosine similarity
/// reduces to standard dot product.
///
/// Returns top-K pairs of `(row_offset, score)` sorted descending by similarity score,
/// with ties broken deterministically by `row_offset ASC`.
pub fn search_cosine_top_k(
    query_vec: &[f32],
    storage: &MmapVectorStorage,
    top_k: usize,
) -> Vec<(usize, f32)> {
    let count = storage.count();
    let dim = storage.dimension();

    if count == 0 || top_k == 0 {
        return Vec::new();
    }

    if query_vec.len() != dim {
        return Vec::new();
    }

    // Ensure query vector is unit normalized
    let norm_query = normalized_l2(query_vec);
    let all_floats = storage.all_vectors();

    // Parallel scan across all vector rows
    let mut scores: Vec<(usize, f32)> = (0..count)
        .into_par_iter()
        .map(|i| {
            let start = i * dim;
            let end = start + dim;
            let vec_slice = &all_floats[start..end];
            let score = dot_product_simd(&norm_query, vec_slice);
            (i, score)
        })
        .collect();

    if scores.len() > top_k {
        scores.select_nth_unstable_by(top_k, |a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        scores.truncate(top_k);
    }

    scores.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(&b.0))
    });

    scores
}
