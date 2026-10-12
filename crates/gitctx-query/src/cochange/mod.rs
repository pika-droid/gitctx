//! Co-change graph and impact analysis engine per Phase 7 (GRAPH-01..05).
//!
//! Provides commit-size filtered co-change extraction, exponential time-decay
//! weighted association metrics (support, confidence, lift), petgraph-backed
//! in-memory coupling graph, and composite impact risk scoring.

pub mod config;
pub mod decay;
pub mod extractor;
pub mod graph;
pub mod impact;
pub mod metrics;
pub mod service;

pub use config::{
    CoChangeConfig, DEFAULT_BOT_PATTERNS, DEFAULT_DECAY_HALF_LIFE_DAYS, DEFAULT_MAX_COMMIT_FILES,
    DEFAULT_MIN_SUPPORT,
};
pub use decay::time_decay_weight;
pub use extractor::{CoChangeExtractor, ExtractionStats, RawCoChangePair};
pub use graph::{CoChangeEdge, CoChangeGraph, CoupledEntity, EntityNode, EntityType};
pub use impact::{ImpactAnalyzer, ImpactResult};
pub use metrics::{CoChangeMetrics, MetricsComputer};
pub use service::{GlobalImpactSummary, ImpactService};
