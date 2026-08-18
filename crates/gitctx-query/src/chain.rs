use crate::citation::CitationBuilder;
use gitctx_core::error::CoreError;
use gitctx_core::model::Citation;
use gitctx_index::storage::TouchRecord;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Chronological evolutionary touch node in a function's lineage chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineageChainNode {
    pub touch: TouchRecord,
    pub commit_author: String,
    pub commit_authored_at: i64,
    pub topo_order: u32,
    pub commit_message: String,
    pub citation: Citation,
}

/// Complete reconstructed evolutionary lineage chain for a function per D-16 and INDEX-06.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LineageChain {
    pub lineage_id: u64,
    pub current_path: String,
    pub current_qualified_name: String,
    pub language: String,
    pub nodes: Vec<LineageChainNode>,
}

/// Single-query deterministic lineage chain assembler ordered by `topo_order ASC` per D-16 and INDEX-06.
pub struct ChainAssembler;

impl ChainAssembler {
    /// Retrieve full chronological timeline for `lineage_id` using a single indexed SQLite query.
    pub fn assemble_lineage_chain(
        conn: &Connection,
        lineage_id: u64,
    ) -> Result<LineageChain, CoreError> {
        // 1. Fetch lineage head metadata
        let mut lineage_stmt = conn
            .prepare_cached(
                "SELECT lineage_id, current_path, current_qualified_name, language
             FROM function_lineage
             WHERE lineage_id = ?1;",
            )
            .map_err(|e| CoreError::InvariantViolation(format!("Lineage query error: {e}")))?;

        let lineage_meta = lineage_stmt
            .query_row(params![lineage_id], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| {
                CoreError::InvariantViolation(format!("Failed to find lineage {lineage_id}: {e}"))
            })?;

        // 2. Single indexed SQL query joining function_touches and commits ordered by topo_order ASC
        let mut touches_stmt = conn
            .prepare_cached(
                "SELECT 
                ft.touch_id,
                ft.lineage_id,
                ft.commit_hash,
                ft.path,
                ft.qualified_name,
                ft.arity,
                ft.change_type,
                ft.signature_before,
                ft.signature_after,
                ft.fingerprint,
                ft.chunk_text,
                ft.parse_status,
                ft.start_line,
                ft.end_line,
                c.author,
                c.authored_at,
                c.topo_order,
                c.message
            FROM function_touches ft
            JOIN commits c ON ft.commit_hash = c.hash
            WHERE ft.lineage_id = ?1
            ORDER BY c.topo_order ASC, ft.touch_id ASC;",
            )
            .map_err(|e| CoreError::InvariantViolation(format!("Touches query error: {e}")))?;

        let rows = touches_stmt
            .query_map(params![lineage_id], |row| {
                let start_line_i64: i64 = row.get(12)?;
                let end_line_i64: i64 = row.get(13)?;
                let topo_order_u32: u32 = row.get(16)?;

                let touch = TouchRecord {
                    touch_id: row.get(0)?,
                    lineage_id: row.get(1)?,
                    commit_hash: row.get(2)?,
                    path: row.get(3)?,
                    qualified_name: row.get(4)?,
                    arity: row.get(5)?,
                    change_type: row.get(6)?,
                    signature_before: row.get(7)?,
                    signature_after: row.get(8)?,
                    fingerprint: row.get(9)?,
                    chunk_text: row.get(10)?,
                    parse_status: row.get(11)?,
                    start_line: start_line_i64 as usize,
                    end_line: end_line_i64 as usize,
                };

                let author: String = row.get(14)?;
                let authored_at: i64 = row.get(15)?;
                let message: String = row.get(17)?;

                Ok((touch, author, authored_at, topo_order_u32, message))
            })
            .map_err(|e| CoreError::InvariantViolation(format!("Touches map error: {e}")))?;

        let mut nodes = Vec::new();
        for r in rows {
            let (touch, author, authored_at, topo_order, message) =
                r.map_err(|e| CoreError::InvariantViolation(format!("Row error: {e}")))?;

            let citation = CitationBuilder::from_touch(&touch, "lineage_chain", 1.0)?;

            nodes.push(LineageChainNode {
                touch,
                commit_author: author,
                commit_authored_at: authored_at,
                topo_order,
                commit_message: message,
                citation,
            });
        }

        Ok(LineageChain {
            lineage_id: lineage_meta.0,
            current_path: lineage_meta.1,
            current_qualified_name: lineage_meta.2,
            language: lineage_meta.3,
            nodes,
        })
    }
}
