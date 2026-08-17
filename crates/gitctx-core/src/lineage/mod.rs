//! Function evolutionary lineage engine and mutation tracking.
//!
//! Provides deterministic 128-permutation MinHash fingerprinting, lexical AST normalization,
//! multi-tier lineage resolution, greedy bipartite matching, and derivation DAG graph.

pub mod dag;
pub mod matcher;
pub mod minhash;
pub mod normalizer;
pub mod resolver;
pub mod seeds;

pub use dag::{DerivationKind, LineageDag, LineageEdge};
pub use matcher::{
    BipartiteMatcher, BipartiteSolution, CandidateGenerator, CandidateMatch, MatchThresholds,
    MatchTier, SymbolDescriptor,
};
pub use minhash::{
    murmur3_32, MinHashFingerprint, MINHASH_PERMUTATIONS, SHINGLE_K, SHORT_FUNCTION_TOKEN_THRESHOLD,
};
pub use normalizer::AstNormalizer;
pub use resolver::{normalize_path, LineageResolver, ResolvedCommit, ResolvedTouch};
pub use seeds::MINHASH_SEEDS;
