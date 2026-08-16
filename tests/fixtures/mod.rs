//! Test fixtures module for gitctx.
//!
//! Provides deterministic Git repository generation, scenario presets,
//! and sample multi-language code snippets.

#![allow(dead_code, unused_imports)]

pub mod languages;
pub mod mkrepo;
pub mod scenarios;

pub use mkrepo::RepoBuilder;
