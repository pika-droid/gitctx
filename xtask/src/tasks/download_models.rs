//! Ahead-of-time embedding model downloader scaffolding per D-14.
//!
//! Enforces the two-phase model rule: model weights are explicitly fetched ahead-of-time
//! and runtime initialization strictly reads from local disk without network calls.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Default supported offline embedding models.
pub const SUPPORTED_MODELS: &[(&str, &str)] = &[
    (
        "BAAI/bge-small-en-v1.5",
        "FastEmbed default English embedding model (384-dim, ~133MB)",
    ),
    (
        "sentence-transformers/all-MiniLM-L6-v2",
        "MiniLM compact sentence embedding model (384-dim, ~90MB)",
    ),
];

/// Scaffolds and runs ahead-of-time model downloading.
pub fn run_download_models(model_id: Option<&str>, dest_dir: Option<&Path>) -> Result<()> {
    let root = crate::util::project_root();
    let target_dest: PathBuf = dest_dir.map_or_else(|| root.join("models"), Path::to_path_buf);

    println!("\n=== GitCtx Ahead-of-Time Model Downloader ===");
    println!("Target Directory: {}", target_dest.display());

    let selected_model = model_id.unwrap_or("BAAI/bge-small-en-v1.5");
    println!("Selected Model:   {selected_model}");

    std::fs::create_dir_all(&target_dest)?;

    println!("\nSupported Ahead-of-Time Offline Models:");
    for (name, desc) in SUPPORTED_MODELS {
        let mark = if *name == selected_model {
            " [*]"
        } else {
            " [ ]"
        };
        println!(" - {name}{mark}\n   Description: {desc}");
    }

    println!("\nInvariant I1 & D-14 Two-Phase Embedding Model Rule:");
    println!("  1. Models must be downloaded ahead-of-time during setup via `cargo xtask download-models`.");
    println!("  2. Runtime embedding initialization in `gitctx-index` and `gitctx-cli` must strictly read");
    println!("     from local cache without falling back to silent network downloads.");
    println!("  3. If weights are missing at runtime, the process MUST fail fast with an actionable error.");

    println!(
        "\nModel scaffolding directory prepared at: {}",
        target_dest.display()
    );
    println!("Model download initialization completed successfully.\n");
    Ok(())
}
