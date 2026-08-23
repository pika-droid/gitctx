use crate::envelope::ResponseEnvelope;
use crate::Format;
use anyhow::Context;
use gitctx_core::config::IndexConfig;
use gitctx_index::pipeline::IndexPipeline;
use gitctx_index::storage::CURRENT_SCHEMA_VERSION;
use std::path::Path;
use std::time::Instant;

pub fn run(repo_path: &str, format: Format) -> anyhow::Result<()> {
    let start_time = Instant::now();
    let path = Path::new(repo_path);

    // In a real scenario, this config would be loaded from a config file or CLI args
    let config = IndexConfig::new(
        CURRENT_SCHEMA_VERSION,
        "BGESmallENV15", // Model ID
        "code",          // Tokenizer
        500,             // max commit files
        1,               // parser version
    );

    if format == Format::Table {
        println!("Indexing repository at: {}", path.display());
    }

    IndexPipeline::run(path, &config).context("Failed to run indexing pipeline")?;

    let duration = start_time.elapsed();

    if format == Format::Json {
        let env = ResponseEnvelope::success(serde_json::json!({
            "status": "success",
            "repo_path": repo_path,
            "duration_ms": duration.as_millis(),
        }));
        println!("{}", serde_json::to_string_pretty(&env)?);
    } else {
        println!("Successfully indexed repository in {duration:?}");
    }

    Ok(())
}
