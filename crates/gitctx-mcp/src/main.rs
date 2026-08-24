#![forbid(unsafe_code)]

//! `gitctx-mcp` — Model Context Protocol stdio server for gitctx.
//!
//! Exposes `search_history`, `function_evolution`, `change_impact`,
//! `who_knows_this`, and `commit_detail` as MCP tools over stdin/stdout
//! JSON-RPC 2.0. All logging goes to stderr per MCP-03.

mod budget;
mod protocol;
mod server;
mod tools;

use std::path::Path;
use tools::ToolContext;

fn main() {
    // Route ALL tracing/logging to stderr — stdout is reserved for JSON-RPC per MCP-03.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();

    eprintln!("[gitctx-mcp] starting stdio server");

    // Discover repository root from current working directory
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

    let ctx = match ToolContext::open(&cwd) {
        Ok(ctx) => {
            eprintln!("[gitctx-mcp] index loaded from {}", cwd.display());
            ctx
        }
        Err(e) => {
            eprintln!("[gitctx-mcp] warning: {e}");
            eprintln!("[gitctx-mcp] server will start but tools may fail until 'gitctx index' is run");
            // Try to start anyway — tools will return appropriate errors
            match ToolContext::open(&cwd) {
                Ok(ctx) => ctx,
                Err(_) => {
                    // Cannot open at all — run a stub server that returns errors for tools
                    eprintln!("[gitctx-mcp] fatal: cannot initialize. Ensure 'gitctx index' has been run in this directory.");
                    std::process::exit(1);
                }
            }
        }
    };

    // Run the synchronous stdio server loop
    server::run_server(ctx);
}
