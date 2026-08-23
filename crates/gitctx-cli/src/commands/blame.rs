use crate::envelope::ResponseEnvelope;
use crate::Format;
use anyhow::{Context, Result};
use std::path::Path;
use std::time::Instant;

pub fn run(file_path: &str, format: Format) -> Result<()> {
    let start_time = Instant::now();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

    let repo =
        git2::Repository::discover(&current_dir).context("Failed to discover git repository")?;

    let blame = repo
        .blame_file(Path::new(file_path), None)
        .context("Failed to blame file")?;

    let mut lines = Vec::new();
    for hunk in blame.iter() {
        let commit_id = hunk.final_commit_id();
        let signature = hunk.final_signature();
        let author = signature.name().unwrap_or("Unknown");
        let start_line = hunk.final_start_line();
        let line_count = hunk.lines_in_hunk();

        lines.push(serde_json::json!({
            "commit": commit_id.to_string(),
            "author": author,
            "start_line": start_line,
            "line_count": line_count,
        }));
    }

    if format == Format::Json {
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": "success",
            "file": file_path,
            "blame": lines,
            "duration_ms": start_time.elapsed().as_millis(),
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        println!("Blame for {file_path}:");
        for hunk in blame.iter() {
            let commit_id = hunk.final_commit_id();
            let author = hunk
                .final_signature()
                .name()
                .unwrap_or("Unknown")
                .to_string();
            let start = hunk.final_start_line();
            let count = hunk.lines_in_hunk();

            // Format commit hash to 8 chars
            let short_hash = commit_id.to_string().chars().take(8).collect::<String>();

            for i in 0..count {
                println!("{} ({}) | Line {}", short_hash, author, start + i);
            }
        }
    }

    Ok(())
}
