//! Synthetic mutation testing framework (M1-M10).
//!
//! Exposes transformation classes, execution engine, and ground-truth
//! lineage manifests for empirical evaluation of function lineage tracking.

pub mod engine;
pub mod manifest;
pub mod transformations;

pub use engine::{MutationEngine, MutationError};
pub use manifest::{GroundTruthRelation, LineageGroundTruthManifest, LineageLink, SymbolLocation};
pub use transformations::MutationClass;
