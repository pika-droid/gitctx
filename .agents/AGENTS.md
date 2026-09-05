# Agent Configuration

<!-- gitctx-begin -->
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
<!-- gitctx-end -->
