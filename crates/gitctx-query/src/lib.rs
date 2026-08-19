#![deny(unsafe_code)]

//! `gitctx-query` provides hybrid search retrieval, Reciprocal Rank Fusion (RRF),
//! exact symbol fast-path, chunk deduplication, commit density capping, deterministic
//! lineage chain assembly, structured citation metadata, and co-change coupling analysis.

pub use gitctx_core as core;
pub use gitctx_index as index;

pub mod chain;
pub mod citation;
pub mod cochange;
pub mod retrieval;
pub mod rrf;
pub mod symbol;

pub use chain::{ChainAssembler, LineageChain, LineageChainNode};
pub use citation::CitationBuilder;
pub use cochange::{
    CoChangeConfig, CoChangeEdge, CoChangeExtractor, CoChangeGraph, CoChangeMetrics, CoupledEntity,
    EntityType, ExtractionStats, ImpactAnalyzer, ImpactResult, MetricsComputer, RawCoChangePair,
};
pub use retrieval::{
    deduplicate_and_cap_touches, HybridRetriever, RetrievedFact, DEFAULT_MAX_TOUCHES_PER_COMMIT,
};
pub use rrf::{fuse_rankings, RrfCandidate, RrfParams, RRF_DEFAULT_K};
pub use symbol::ExactSymbolMatcher;
