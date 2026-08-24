use gitctx_core::config::IndexConfig;
use gitctx_index::pipeline::IndexPipeline;
use gitctx_index::storage::CURRENT_SCHEMA_VERSION;
use serde_json::Value;

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use tempfile::TempDir;

fn setup_repo() -> TempDir {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let path = temp_dir.path();

    // Initialize git repo
    Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .expect("git init failed");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(path)
        .output()
        .expect("git config failed");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .expect("git config failed");

    // Write a file
    let src_dir = path.join("src");
    fs::create_dir(&src_dir).expect("failed to create src dir");
    fs::write(
        src_dir.join("main.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )
    .expect("failed to write file");

    // Commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .expect("git add failed");
    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(path)
        .output()
        .expect("git commit failed");

    // Index repository directly in-process
    let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "BGESmallENV15", "code", 500, 1);
    IndexPipeline::run(path, &config).expect("IndexPipeline::run failed");

    temp_dir
}

#[test]
fn test_mcp_inspector() {
    let repo_dir = setup_repo();
    let mcp_bin = env!("CARGO_BIN_EXE_gitctx-mcp");

    let mut mcp_process = Command::new(mcp_bin)
        .current_dir(repo_dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn gitctx-mcp");

    let mut stdin = mcp_process.stdin.take().expect("Failed to open stdin");
    let stdout = mcp_process.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // 1. Send initialize
    let init_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&init_req).unwrap()).unwrap();

    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("Failed to read initialize response");
    let init_resp: Value = serde_json::from_str(&line).expect("Invalid JSON");
    assert_eq!(init_resp["jsonrpc"], "2.0");
    assert_eq!(init_resp["id"], 1);
    assert!(init_resp["result"]["capabilities"].is_object());
    assert_eq!(init_resp["result"]["serverInfo"]["name"], "gitctx");

    // 2. Send tools/list
    let list_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    writeln!(stdin, "{}", serde_json::to_string(&list_req).unwrap()).unwrap();

    line.clear();
    reader
        .read_line(&mut line)
        .expect("Failed to read tools/list response");
    let list_resp: Value = serde_json::from_str(&line).expect("Invalid JSON");
    assert_eq!(list_resp["id"], 2);
    let tools = list_resp["result"]["tools"]
        .as_array()
        .expect("Tools should be array");
    assert_eq!(tools.len(), 5);

    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"search_history"));
    assert!(tool_names.contains(&"function_evolution"));
    assert!(tool_names.contains(&"change_impact"));
    assert!(tool_names.contains(&"who_knows_this"));
    assert!(tool_names.contains(&"commit_detail"));

    // 3. Send tools/call (search_history)
    let call_req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "search_history",
            "arguments": {
                "query": "main",
                "mode": "compact"
            }
        }
    });
    writeln!(stdin, "{}", serde_json::to_string(&call_req).unwrap()).unwrap();

    line.clear();
    reader
        .read_line(&mut line)
        .expect("Failed to read tools/call response");
    let call_resp: Value = serde_json::from_str(&line).expect("Invalid JSON");
    assert_eq!(call_resp["id"], 3);

    // Parse the inner JSON response (it's embedded as text per MCP spec)
    let content = call_resp["result"]["content"][0]["text"]
        .as_str()
        .expect("Expected text content");
    let inner_resp: Value = serde_json::from_str(content).expect("Invalid inner JSON");

    assert!(
        !inner_resp["results"].as_array().unwrap().is_empty(),
        "Should find at least 1 result for 'main'"
    );

    // Cleanup
    drop(stdin);
    let _ = mcp_process.kill();
    let _ = mcp_process.wait();
}
