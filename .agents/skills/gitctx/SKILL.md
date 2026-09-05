---
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
