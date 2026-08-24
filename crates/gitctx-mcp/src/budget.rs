//! Token budgeting for MCP responses per MCP-02.
//!
//! - `compact` (default): citation-only mode — returns commit_hash, path, start_line,
//!   end_line, match_rule, match_confidence, qualified_name. No chunk_text.
//! - `full`: includes complete chunk_text/diff content. Requires explicit `mode: "full"`.

use gitctx_query::chain::{LineageChain, LineageChainNode};
use gitctx_query::retrieval::RetrievedFact;
use serde::{Deserialize, Serialize};

/// Budget mode for token-aware MCP responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BudgetMode {
    #[default]
    Compact,
    Full,
}

impl BudgetMode {
    /// Parse from optional string, defaulting to Compact.
    pub fn from_str_opt(s: Option<&str>) -> Self {
        match s {
            Some("full") => Self::Full,
            _ => Self::Compact,
        }
    }
}

/// Format a retrieved fact according to the budget mode.
pub fn format_fact(fact: &RetrievedFact, mode: BudgetMode) -> serde_json::Value {
    match mode {
        BudgetMode::Compact => serde_json::json!({
            "commit_hash": fact.commit_hash,
            "path": fact.path,
            "qualified_name": fact.qualified_name,
            "change_type": fact.change_type,
            "score": fact.score,
            "rank": fact.rank,
            "citation": {
                "commit_hash": fact.citation.commit_hash,
                "path": fact.citation.path,
                "start_line": fact.citation.start_line,
                "end_line": fact.citation.end_line,
                "match_rule": fact.citation.match_rule,
                "match_confidence": fact.citation.match_confidence,
            }
        }),
        BudgetMode::Full => serde_json::json!({
            "touch_id": fact.touch_id,
            "lineage_id": fact.lineage_id,
            "commit_hash": fact.commit_hash,
            "path": fact.path,
            "qualified_name": fact.qualified_name,
            "change_type": fact.change_type,
            "chunk_text": fact.chunk_text,
            "score": fact.score,
            "rank": fact.rank,
            "citation": {
                "commit_hash": fact.citation.commit_hash,
                "path": fact.citation.path,
                "start_line": fact.citation.start_line,
                "end_line": fact.citation.end_line,
                "match_rule": fact.citation.match_rule,
                "match_confidence": fact.citation.match_confidence,
            }
        }),
    }
}

/// Format a lineage chain node according to the budget mode.
pub fn format_chain_node(node: &LineageChainNode, mode: BudgetMode) -> serde_json::Value {
    match mode {
        BudgetMode::Compact => serde_json::json!({
            "commit_hash": node.touch.commit_hash,
            "path": node.touch.path,
            "qualified_name": node.touch.qualified_name,
            "change_type": node.touch.change_type,
            "author": node.commit_author,
            "authored_at": node.commit_authored_at,
            "message_headline": node.commit_message.lines().next().unwrap_or(""),
            "citation": {
                "start_line": node.citation.start_line,
                "end_line": node.citation.end_line,
            }
        }),
        BudgetMode::Full => serde_json::json!({
            "commit_hash": node.touch.commit_hash,
            "path": node.touch.path,
            "qualified_name": node.touch.qualified_name,
            "change_type": node.touch.change_type,
            "chunk_text": node.touch.chunk_text,
            "author": node.commit_author,
            "authored_at": node.commit_authored_at,
            "message": node.commit_message,
            "topo_order": node.topo_order,
            "citation": {
                "start_line": node.citation.start_line,
                "end_line": node.citation.end_line,
                "match_rule": node.citation.match_rule,
                "match_confidence": node.citation.match_confidence,
            }
        }),
    }
}

/// Format a complete lineage chain according to the budget mode.
pub fn format_chain(chain: &LineageChain, mode: BudgetMode) -> serde_json::Value {
    serde_json::json!({
        "lineage_id": chain.lineage_id,
        "current_path": chain.current_path,
        "current_qualified_name": chain.current_qualified_name,
        "language": chain.language,
        "touch_count": chain.nodes.len(),
        "nodes": chain.nodes.iter().map(|n| format_chain_node(n, mode)).collect::<Vec<_>>(),
    })
}
