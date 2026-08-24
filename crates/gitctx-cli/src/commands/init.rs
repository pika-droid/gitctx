use crate::Format;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const MARKER_BEGIN: &str = "<!-- gitctx-begin -->";
const MARKER_END: &str = "<!-- gitctx-end -->";

const AGENTS_MD_BLOCK: &str = r#"<!-- gitctx-begin -->
## gitctx — Git History Intelligence

### MCP Server Configuration

```json
{
  "mcpServers": {
    "gitctx": {
      "command": "gitctx-mcp",
      "args": [],
      "cwd": "."
    }
  }
}
```

### Available Tools

| Tool | Purpose | When to Use |
|------|---------|-------------|
| `search_history` | Search indexed Git history for functions, changes, and commits | When investigating why code changed, finding related changes, understanding rationale |
| `function_evolution` | Track complete evolutionary history of a function across renames/moves | When understanding how a function evolved, tracking refactors, reviewing design changes |
| `change_impact` | Analyze risk and co-change coupling for a file or function | Before modifying coupled code, risk assessment, identifying blast radius |
| `who_knows_this` | Find top contributors who modified a function or file | Identifying domain experts, code owners, reviewers |
| `commit_detail` | Show detailed commit info with all functions touched | Deep-diving into a specific commit, understanding change scope |

### Usage Guidelines

- **Default to `compact` mode** — only use `mode: "full"` when you need actual diff/chunk text content.
- **Use `search_history`** for why-questions ("why was this changed?", "what commits affected authentication?").
- **Use `function_evolution`** for tracking a specific function's timeline.
- **Use `change_impact`** before modifying tightly-coupled code.
- **Use `who_knows_this`** to identify reviewers or domain experts.

### Negative Guidance (Do NOT)

- ❌ Do NOT use `search_history` for current file content — use your IDE/editor instead.
- ❌ Do NOT use `function_evolution` for functions that don't exist in the index — check with `search_history` first.
- ❌ Do NOT use `change_impact` on the entire repository — target specific files or functions.
- ❌ Do NOT call tools repeatedly with the same query — results are deterministic.
- ❌ Do NOT assume the index is always fresh — check `index_status.stale_index` in responses.
<!-- gitctx-end -->"#;

const SKILL_MD: &str = r#"---
name: gitctx
description: Git history intelligence — search history, track function evolution, analyze change impact, and find domain experts using the gitctx MCP server.
---

# gitctx Skill

## Overview

gitctx provides offline, deterministic Git history intelligence through five MCP tools. It indexes your repository's commit history, function-level changes, and co-change patterns.

## Quick Start

1. **Index the repository**: Run `gitctx index` in the repo root.
2. **Use MCP tools**: The `gitctx-mcp` server exposes tools over stdio JSON-RPC.

## Tool Patterns

### Find why code changed
```
search_history(query="authentication token validation")
```

### Track a function's history
```
function_evolution(identifier="parse_config")
function_evolution(identifier="Config::parse")
```

### Check change risk before modifying
```
change_impact(target="src/auth/token.rs")
change_impact(target="validate_token")
```

### Find who knows this code
```
who_knows_this(identifier="src/core/engine.rs")
who_knows_this(identifier="HybridRetriever::search")
```

### Inspect a specific commit
```
commit_detail(hash="abc123")
commit_detail(hash="abc123", mode="full")  # includes chunk text
```

## Token Budget

- `compact` (default): Returns citations only — commit hash, path, line range, match rule. Minimal tokens.
- `full`: Includes full chunk text and diff content. Use sparingly.

## Stale Index Warning

Tool responses may include `index_status.stale_index: true` if HEAD has advanced past the last indexed commit. Re-run `gitctx index` to update.
"#;

pub fn run(_format: Format) -> Result<()> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;

    // Create .agents directory
    let agents_dir = current_dir.join(".agents");
    fs::create_dir_all(&agents_dir).context("Failed to create .agents directory")?;

    // Write/merge AGENTS.md
    let agents_path = agents_dir.join("AGENTS.md");
    write_with_markers(&agents_path, AGENTS_MD_BLOCK)?;
    println!("✓ Updated {}", agents_path.display());

    // Create skill directory
    let skill_dir = agents_dir.join("skills").join("gitctx");
    fs::create_dir_all(&skill_dir).context("Failed to create skill directory")?;

    // Write SKILL.md (always overwrite — it's fully owned by gitctx)
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, SKILL_MD).context("Failed to write SKILL.md")?;
    println!("✓ Updated {}", skill_path.display());

    println!("\ngitctx init complete. MCP server available via 'gitctx-mcp'.");

    Ok(())
}

/// Write content with marker-based idempotent merging.
/// If the file already contains the markers, replace only that section.
/// If the file exists but has no markers, append the content.
/// If the file doesn't exist, create it with the content.
fn write_with_markers(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        let existing = fs::read_to_string(path).context("Failed to read existing file")?;

        if let (Some(start), Some(end)) = (existing.find(MARKER_BEGIN), existing.find(MARKER_END)) {
            // Replace existing gitctx section
            let end_marker_end = end + MARKER_END.len();
            let mut new_content = String::new();
            new_content.push_str(&existing[..start]);
            new_content.push_str(content);
            if end_marker_end < existing.len() {
                new_content.push_str(&existing[end_marker_end..]);
            }
            fs::write(path, new_content).context("Failed to write updated file")?;
        } else {
            // Append gitctx section
            let mut new_content = existing;
            if !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            new_content.push('\n');
            new_content.push_str(content);
            new_content.push('\n');
            fs::write(path, new_content).context("Failed to append to file")?;
        }
    } else {
        // Create new file
        let mut new_content = String::from("# Agent Configuration\n\n");
        new_content.push_str(content);
        new_content.push('\n');
        fs::write(path, new_content).context("Failed to create file")?;
    }

    Ok(())
}
