use gitctx_core::config::IndexConfig;
use gitctx_index::storage::{initialize_schema, open_connection, CURRENT_SCHEMA_VERSION};
use gitctx_index::tantivy_index::schema::TantivyTouchDoc;
use gitctx_index::tantivy_index::TantivyEngine;
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

    let repo = git2::Repository::open(path).expect("open git repo");
    let head_commit = repo
        .head()
        .expect("head")
        .target()
        .expect("target")
        .to_string();

    // Set up .gitctx structure directly without ONNX runtime in test process
    let gitctx_dir = path.join(".gitctx");
    fs::create_dir_all(&gitctx_dir).expect("create .gitctx dir");
    let tantivy_dir = gitctx_dir.join("tantivy");
    fs::create_dir_all(&tantivy_dir).expect("create tantivy dir");

    let db_path = gitctx_dir.join("index.db");
    let conn = open_connection(&db_path).expect("open index db");
    initialize_schema(&conn).expect("initialize schema");

    let config = IndexConfig::new(CURRENT_SCHEMA_VERSION, "BGESmallENV15", "code", 500, 1);
    conn.execute(
        "INSERT INTO index_state (key, value) VALUES ('schema_version', ?1), ('config_hash', ?2), ('head_at_index', ?3)",
        rusqlite::params![
            CURRENT_SCHEMA_VERSION.to_string(),
            config.compute_config_hash().as_str(),
            head_commit
        ],
    )
    .expect("insert index_state");

    conn.execute(
        "INSERT INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
         VALUES (?1, 'Test User', 'test@example.com', 1700000000, 1, 'main', '', 'Initial commit', 3, 0)",
        rusqlite::params![head_commit],
    )
    .expect("insert commit");

    conn.execute(
        "INSERT INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
         VALUES (1, 'src/main.rs', 'main', 'rust')",
        [],
    )
    .expect("insert lineage");

    conn.execute(
        "INSERT INTO function_touches (touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type, signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line)
         VALUES (1, 1, ?1, 'src/main.rs', 'main', 0, 'add', NULL, 'fn main()', NULL, 'fn main() {\n    println!(\"Hello, world!\");\n}', 'ok', 1, 3)",
        rusqlite::params![head_commit],
    )
    .expect("insert touch");

    drop(conn);

    let mut tantivy =
        TantivyEngine::create_in_dir(&tantivy_dir, 15_000_000).expect("create tantivy");
    tantivy
        .add_touch(&TantivyTouchDoc {
            touch_id: 1,
            lineage_id: 1,
            commit_hash: head_commit,
            qualified_name: "main".to_string(),
            path: "src/main.rs".to_string(),
            commit_message: "Initial commit".to_string(),
            chunk_text: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
        })
        .expect("add touch doc");
    tantivy.commit().expect("tantivy commit");

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
