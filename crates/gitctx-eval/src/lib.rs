#![forbid(unsafe_code)]

//! `gitctx-eval` provides synthetic mutation testing (M1-M10), benchmark metrics, and evaluation harnesses.

#[path = "../../../tests/fixtures/mod.rs"]
pub mod fixtures;

pub mod corpus;
pub mod manifest;
pub mod mutator;
pub mod oracle;
pub mod report;

pub use corpus::*;
pub use fixtures::mkrepo::RepoBuilder;
pub use gitctx_core as core;
pub use gitctx_index as index;
pub use gitctx_query as query;
pub use manifest::*;
pub use mutator::*;
pub use oracle::*;
