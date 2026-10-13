#![deny(unsafe_code)]

//! `gitctx-index` provides persistent index storage, SQLite relations, Tantivy indices, and vector embeddings.

pub use gitctx_core as core;

pub mod pipeline;
pub mod sink;
pub mod storage;
pub mod tantivy_index;
pub mod vector;
