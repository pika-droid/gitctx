//! 128-Permutation MinHash fingerprinting and pure-Rust Murmur3-32 hashing per D-02, D-03, D-04.

use crate::lineage::seeds::MINHASH_SEEDS;
use serde::{Deserialize, Serialize};

/// Number of MinHash permutations per fingerprint (512 bytes).
pub const MINHASH_PERMUTATIONS: usize = 128;

/// Shingle window size (k=3 3-grams).
pub const SHINGLE_K: usize = 3;

/// Short function length gating threshold (< 10 normalized tokens).
pub const SHORT_FUNCTION_TOKEN_THRESHOLD: usize = 10;

/// Pure Rust, zero-dependency, platform-independent Murmur3 32-bit hash.
#[inline]
pub fn murmur3_32(data: &[u8], seed: u32) -> u32 {
    const C1: u32 = 0xcc9e2d51;
    const C2: u32 = 0x1b873593;
    let mut h = seed;
    let chunks = data.chunks_exact(4);
    let remainder = chunks.remainder();

    for chunk in chunks {
        let mut k = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        k = k.wrapping_mul(C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(C2);
        h ^= k;
        h = h.rotate_left(13);
        h = h.wrapping_mul(5).wrapping_add(0xe6546b64);
    }

    let mut k = 0u32;
    if remainder.len() == 3 {
        k ^= (remainder[2] as u32) << 16;
    }
    if remainder.len() >= 2 {
        k ^= (remainder[1] as u32) << 8;
    }
    if remainder.len() >= 1 {
        k ^= remainder[0] as u32;
        k = k.wrapping_mul(C1);
        k = k.rotate_left(15);
        k = k.wrapping_mul(C2);
        h ^= k;
    }

    h ^= data.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h
}

/// A 128-permutation MinHash fingerprint (512 bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MinHashFingerprint(#[serde(with = "serde_arrays")] pub [u32; MINHASH_PERMUTATIONS]);

mod serde_arrays {
    use super::MINHASH_PERMUTATIONS;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(data: &[u32; MINHASH_PERMUTATIONS], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        data.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u32; MINHASH_PERMUTATIONS], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec = Vec::<u32>::deserialize(deserializer)?;
        if vec.len() == MINHASH_PERMUTATIONS {
            let mut arr = [0u32; MINHASH_PERMUTATIONS];
            arr.copy_from_slice(&vec);
            Ok(arr)
        } else {
            Err(serde::de::Error::custom(format!(
                "expected array of length {MINHASH_PERMUTATIONS}, got {}",
                vec.len()
            )))
        }
    }
}

impl Default for MinHashFingerprint {
    fn default() -> Self {
        Self([0u32; MINHASH_PERMUTATIONS])
    }
}

impl MinHashFingerprint {
    /// Compute estimated Jaccard similarity between two fingerprints in [0.0, 1.0].
    #[must_use]
    pub fn similarity(&self, other: &Self) -> f32 {
        let matching = self
            .0
            .iter()
            .zip(other.0.iter())
            .filter(|(a, b)| a == b)
            .count();
        matching as f32 / MINHASH_PERMUTATIONS as f32
    }

    /// Construct a MinHash fingerprint from normalized token shingles.
    ///
    /// Returns `(fingerprint, is_short)` where `is_short` is true if the token count < 10.
    pub fn from_tokens(tokens: &[String]) -> (Self, bool) {
        let is_short = tokens.len() < SHORT_FUNCTION_TOKEN_THRESHOLD;
        if tokens.is_empty() {
            return (Self([0u32; MINHASH_PERMUTATIONS]), is_short);
        }

        let mut min_hashes = [u32::MAX; MINHASH_PERMUTATIONS];

        if tokens.len() < SHINGLE_K {
            let shingle = tokens.join(" ");
            let bytes = shingle.as_bytes();
            for i in 0..MINHASH_PERMUTATIONS {
                let h = murmur3_32(bytes, MINHASH_SEEDS[i]);
                min_hashes[i] = h;
            }
        } else {
            for window in tokens.windows(SHINGLE_K) {
                let shingle = window.join(" ");
                let bytes = shingle.as_bytes();
                for i in 0..MINHASH_PERMUTATIONS {
                    let h = murmur3_32(bytes, MINHASH_SEEDS[i]);
                    if h < min_hashes[i] {
                        min_hashes[i] = h;
                    }
                }
            }
        }

        (Self(min_hashes), is_short)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_murmur3_deterministic_vectors() {
        assert_eq!(murmur3_32(b"", 0), 0);
        assert_eq!(murmur3_32(b"hello", 0), 0x248bfa47);
        assert_eq!(murmur3_32(b"hello world", 42), 3926694905);
    }

    #[test]
    fn test_minhash_identical_tokens_similarity() {
        let tokens: Vec<String> = vec![
            "<kw:def>", "var_0", "<op:(>", "var_1", "<op:)>", "<op::>",
            "<kw:return>", "var_1", "<op:+>", "<num>",
        ]
        .into_iter()
        .map(ToString::to_string)
        .collect();

        let (fp1, is_short1) = MinHashFingerprint::from_tokens(&tokens);
        let (fp2, is_short2) = MinHashFingerprint::from_tokens(&tokens);

        assert!(!is_short1);
        assert!(!is_short2);
        assert_eq!(fp1, fp2);
        assert!((fp1.similarity(&fp2) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_short_function_gating() {
        let short_tokens: Vec<String> = vec!["<kw:def>", "var_0", "<kw:return>", "var_1"]
            .into_iter()
            .map(ToString::to_string)
            .collect();

        let (_fp, is_short) = MinHashFingerprint::from_tokens(&short_tokens);
        assert!(is_short);
    }
}
