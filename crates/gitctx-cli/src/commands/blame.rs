use crate::envelope::ResponseEnvelope;
use crate::format::format_blame;
use crate::Format;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameHunkInfo {
    pub commit: String,
    pub author: String,
    pub start_line: usize,
    pub line_count: usize,
}

pub fn run(file_path: &str, format: Format) -> Result<()> {
    let start_time = Instant::now();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

    let repo =
        git2::Repository::discover(&current_dir).context("Failed to discover git repository")?;

    let blame = repo
        .blame_file(Path::new(file_path), None)
        .context("Failed to blame file")?;

    let mut hunks = Vec::new();
    for hunk in blame.iter() {
        let commit_id = hunk.final_commit_id();
        let signature = hunk.final_signature();
        let author = signature.name().unwrap_or("Unknown").to_string();
        let start_line = hunk.final_start_line();
        let line_count = hunk.lines_in_hunk();

        hunks.push(BlameHunkInfo {
            commit: commit_id.to_string(),
            author,
            start_line,
            line_count,
        });
    }

    if format == Format::Json {
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": "success",
            "file": file_path,
            "blame": hunks,
            "duration_ms": start_time.elapsed().as_millis(),
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        format_blame(file_path, &hunks);
    }

    Ok(())
}
