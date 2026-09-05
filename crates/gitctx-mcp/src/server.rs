//! MCP stdio server dispatcher per MCP-01 and MCP-03.
//!
//! Reads JSON-RPC 2.0 messages from stdin line-by-line, dispatches to the appropriate
//! MCP handler, and writes responses to stdout. All logging goes to stderr.

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, INVALID_PARAMS, METHOD_NOT_FOUND};
use crate::tools::{self, ToolContext};
use std::io::{self, BufRead, Write};

/// MCP server info returned during initialization.
const SERVER_NAME: &str = "gitctx";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio MCP server loop.
#[allow(clippy::needless_pass_by_value)]
pub fn run_server(ctx: ToolContext) {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[gitctx-mcp] stdin read error: {e}");
                break;
            }
        };

        // Skip empty lines (JSON-RPC uses \n-delimited messages)
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[gitctx-mcp] JSON parse error: {e}");
                let resp = JsonRpcResponse::error(
                    None,
                    crate::protocol::PARSE_ERROR,
                    format!("Parse error: {e}"),
                );
                write_response(&mut stdout_lock, &resp);
                continue;
            }
        };

        // Dispatch method
        let response = dispatch(&ctx, &request);

        // Only send response if request has an id (JSON-RPC notifications have no id)
        if request.id.is_some() {
            if let Some(resp) = response {
                write_response(&mut stdout_lock, &resp);
            }
        }
    }
}

/// Write a JSON-RPC response as a single line to stdout.
fn write_response(stdout: &mut io::StdoutLock<'_>, response: &JsonRpcResponse) {
    match serde_json::to_string(response) {
        Ok(json) => {
            let _ = writeln!(stdout, "{json}");
            let _ = stdout.flush();
        }
        Err(e) => {
            eprintln!("[gitctx-mcp] Failed to serialize response: {e}");
        }
    }
}

/// Dispatch a JSON-RPC request to the appropriate handler.
fn dispatch(ctx: &ToolContext, request: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    match request.method.as_str() {
        // ── MCP Lifecycle ───────────────────────────────────────────────
        "initialize" => Some(handle_initialize(request)),
        "initialized" => None, // notification, no response
        "ping" => Some(JsonRpcResponse::success(
            request.id.clone(),
            serde_json::json!({}),
        )),

        // ── MCP Tool Discovery ──────────────────────────────────────────
        "tools/list" => Some(handle_tools_list(request)),

        // ── MCP Tool Invocation ─────────────────────────────────────────
        "tools/call" => Some(handle_tools_call(ctx, request)),

        // ── Unknown Method ──────────────────────────────────────────────
        _ => Some(JsonRpcResponse::error(
            request.id.clone(),
            METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
        )),
    }
}

/// Handle MCP `initialize` request — return server capabilities.
fn handle_initialize(request: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::success(
        request.id.clone(),
        serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": SERVER_VERSION,
            }
        }),
    )
}

/// Handle MCP `tools/list` request.
fn handle_tools_list(request: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::success(request.id.clone(), tools::tool_definitions())
}

/// Handle MCP `tools/call` request — dispatch to the appropriate tool.
fn handle_tools_call(ctx: &ToolContext, request: &JsonRpcRequest) -> JsonRpcResponse {
    let tool_name = request
        .params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let arguments = request
        .params
        .get("arguments")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let result = match tool_name {
        "search_history" => ctx.search_history(&arguments),
        "function_evolution" => ctx.function_evolution(&arguments),
        "change_impact" => ctx.change_impact(&arguments),
        "who_knows_this" => ctx.who_knows_this(&arguments),
        "commit_detail" => ctx.commit_detail(&arguments),
        _ => {
            return JsonRpcResponse::error(
                request.id.clone(),
                INVALID_PARAMS,
                format!("Unknown tool: {tool_name}"),
            );
        }
    };

    match result {
        Ok(value) => JsonRpcResponse::success(
            request.id.clone(),
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&value).unwrap_or_default(),
                }],
                "isError": false,
            }),
        ),
        Err(e) => JsonRpcResponse::success(
            request.id.clone(),
            serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": format!("Error: {e}"),
                }],
                "isError": true,
            }),
        ),
    }
}
