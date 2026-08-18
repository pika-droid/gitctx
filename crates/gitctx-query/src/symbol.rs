use regex::Regex;
use rusqlite::{params, Connection};
use std::sync::LazyLock;

static SYMBOL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*|\.[a-zA-Z_][a-zA-Z0-9_]*)*$").unwrap()
});

/// Exact symbol detection heuristic and direct SQLite index lookup engine per D-13 and INDEX-05.
pub struct ExactSymbolMatcher;

impl ExactSymbolMatcher {
    /// Detect if `query` matches code identifier syntax (e.g. `UserAuth::login` or `handle_request`).
    pub fn is_symbol_candidate(query: &str) -> bool {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return false;
        }
        SYMBOL_REGEX.is_match(trimmed)
    }

    /// Query SQLite index on `function_touches(qualified_name)` to retrieve exact matching `touch_id`s per D-13.
    pub fn find_exact_symbol_touches(
        conn: &Connection,
        query: &str,
        limit: usize,
    ) -> rusqlite::Result<Vec<u64>> {
        let trimmed = query.trim();
        if !Self::is_symbol_candidate(trimmed) {
            return Ok(Vec::new());
        }

        let like_suffix = format!("%::{}", trimmed);
        let like_dot = format!("%.{}", trimmed);

        let mut stmt = conn.prepare_cached(
            "SELECT touch_id FROM function_touches 
             WHERE qualified_name = ?1 OR qualified_name LIKE ?2 OR qualified_name LIKE ?3
             ORDER BY touch_id DESC LIMIT ?4;",
        )?;

        let rows = stmt.query_map(
            params![trimmed, like_suffix, like_dot, limit as i64],
            |row| {
                let id: i64 = row.get(0)?;
                Ok(id as u64)
            },
        )?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }
}
