//! Function evolutionary lineage engine and mutation tracking.
//!
//! Provides deterministic 128-permutation MinHash fingerprinting, lexical AST normalization,
//! multi-tier lineage resolution, greedy bipartite matching, and derivation DAG graph.

pub mod minhash;
pub mod normalizer;
pub mod seeds;

pub use minhash::{
    murmur3_32, MinHashFingerprint, MINHASH_PERMUTATIONS, SHINGLE_K,
    SHORT_FUNCTION_TOKEN_THRESHOLD,
};
pub use normalizer::AstNormalizer;
pub use seeds::MINHASH_SEEDS;
