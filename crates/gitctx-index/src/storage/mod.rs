pub mod atomic;
pub mod connection;
pub mod explain;
pub mod schema;
pub mod writer;

pub use atomic::AtomicStorageManager;
pub use connection::{apply_pragmas, open_connection, open_readonly_connection};
pub use explain::ExplainCacheStore;
pub use schema::{initialize_schema, CURRENT_SCHEMA_VERSION, SCHEMA_DDL};
pub use writer::{
    BackgroundWriter, CommitRecord, EmbeddingRecord, ExplainCacheRecord, FileRecord, LineageRecord,
    TouchRecord, WriteMessage,
};

use gitctx_core::config::IndexConfig;
use gitctx_core::error::CoreError;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

/// High-level SQLite Storage Engine for gitctx.
pub struct StorageEngine {
    conn: Connection,
}

impl std::fmt::Debug for StorageEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageEngine").finish()
    }
}

impl StorageEngine {
    /// Open a database connection at `path`, validating schema version and config hash.
    pub fn open<P: AsRef<Path>>(path: P, expected_config: &IndexConfig) -> Result<Self, CoreError> {
        let conn = open_connection(path).map_err(|e| CoreError::RepositoryOpen {
            path: Path::new("index.db").to_path_buf(),
            message: e.to_string(),
        })?;

        // Initialize schema if fresh database
        initialize_schema(&conn).map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        // Validate schema version
        let stored_schema_version: Option<String> = conn
            .query_row(
                "SELECT value FROM index_state WHERE key = 'schema_version';",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        match stored_schema_version {
            Some(v_str) => {
                let version_num: u32 = v_str.parse().unwrap_or(0);
                if version_num != expected_config.schema_version {
                    return Err(CoreError::ConfigMismatch {
                        expected: format!("schema_version={}", expected_config.schema_version),
                        actual: format!("schema_version={version_num}. Index schema version mismatch. Run 'gitctx index' to rebuild."),
                    });
                }
            }
            None => {
                return Err(CoreError::ConfigMismatch {
                    expected: format!("schema_version={}", expected_config.schema_version),
                    actual: "missing schema_version. Index schema version mismatch. Run 'gitctx index' to rebuild.".to_string(),
                });
            }
        }

        // Validate config hash
        let stored_config_hash: Option<String> = conn
            .query_row(
                "SELECT value FROM index_state WHERE key = 'config_hash';",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

        let expected_hash = expected_config.compute_config_hash();
        match stored_config_hash {
            Some(actual_hash) => {
                if actual_hash != expected_hash.as_str() {
                    return Err(CoreError::ConfigMismatch {
                        expected: expected_hash.0,
                        actual: format!("{actual_hash}. Index schema version mismatch. Run 'gitctx index' to rebuild."),
                    });
                }
            }
            None => {
                return Err(CoreError::ConfigMismatch {
                    expected: expected_hash.0,
                    actual: "missing config_hash. Index schema version mismatch. Run 'gitctx index' to rebuild.".to_string(),
                });
            }
        }

        Ok(Self { conn })
    }

    /// Open an unvalidated connection for raw test operations.
    ///
    /// # Safety (logical)
    /// This bypasses `schema_version` and `config_hash` validation (INVAR-03).
    /// **WARNING: Restricted to testing use only. Do not use in production.**
    #[doc(hidden)]
    pub fn open_raw<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        let conn = open_connection(path)?;
        initialize_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Return reference to internal connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Return mutable reference to internal connection.
    pub fn connection_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Record schema version and config hash in index_state.
    pub fn record_state(&self, config: &IndexConfig) -> rusqlite::Result<()> {
        let mut stmt = self
            .conn
            .prepare_cached("INSERT OR REPLACE INTO index_state (key, value) VALUES (?1, ?2);")?;
        stmt.execute(params!["schema_version", config.schema_version.to_string()])?;
        stmt.execute(params![
            "config_hash",
            config.compute_config_hash().as_str()
        ])?;
        Ok(())
    }

    /// Query full evolutionary touch history for a lineage ordered by `topo_order ASC`.
    pub fn query_touches_by_lineage(&self, lineage_id: u64) -> rusqlite::Result<Vec<TouchRecord>> {
        let mut stmt = self.conn.prepare_cached(
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
                ft.end_line
            FROM function_touches ft
            JOIN commits c ON ft.commit_hash = c.hash
            WHERE ft.lineage_id = ?1
            ORDER BY c.topo_order ASC;",
        )?;

        let rows = stmt.query_map(params![lineage_id], |row| {
            let start_line_i64: i64 = row.get(12)?;
            let end_line_i64: i64 = row.get(13)?;
            Ok(TouchRecord {
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
            })
        })?;

        let mut touches = Vec::new();
        for r in rows {
            touches.push(r?);
        }
        Ok(touches)
    }

    /// Query touches matching a qualified name, ordered by topological commit order.
    pub fn query_touches_by_qualname(&self, qualname: &str) -> rusqlite::Result<Vec<TouchRecord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT 
                ft.touch_id, ft.lineage_id, ft.commit_hash, ft.path, ft.qualified_name, ft.arity, ft.change_type,
                ft.signature_before, ft.signature_after, ft.fingerprint, ft.chunk_text, ft.parse_status, ft.start_line, ft.end_line
            FROM function_touches ft
            JOIN commits c ON ft.commit_hash = c.hash
            WHERE ft.qualified_name = ?1
            ORDER BY c.topo_order ASC, ft.touch_id ASC;",
        )?;

        let rows = stmt.query_map(params![qualname], |row| {
            let start_line_i64: i64 = row.get(12)?;
            let end_line_i64: i64 = row.get(13)?;
            Ok(TouchRecord {
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
            })
        })?;

        let mut touches = Vec::new();
        for r in rows {
            touches.push(r?);
        }
        Ok(touches)
    }

    /// Access the physically isolated explain_cache store per D-04 and INDEX-02 (Invariant I5).
    pub fn explain_cache(&self) -> ExplainCacheStore<'_> {
        ExplainCacheStore::new(&self.conn)
    }

    // --- Co-change analysis queries (Phase 7: GRAPH-01..05) ---

    /// Query all commits with their distinct file-touch counts for commit-size filtering.
    ///
    /// Returns `(commit_hash, email, authored_at, file_count)` ordered by `topo_order ASC`.
    pub fn query_commit_file_counts(&self) -> rusqlite::Result<Vec<(String, String, i64, u32)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT c.hash, c.email, c.authored_at, COUNT(DISTINCT ft.path) as file_count
            FROM commits c
            JOIN function_touches ft ON c.hash = ft.commit_hash
            GROUP BY c.hash
            ORDER BY c.topo_order ASC;",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, u32>(3)?,
            ))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// Query distinct file paths touched by a specific commit.
    pub fn query_commit_touched_paths(&self, commit_hash: &str) -> rusqlite::Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT DISTINCT path FROM function_touches WHERE commit_hash = ?1;")?;

        let rows = stmt.query_map(params![commit_hash], |row| row.get::<_, String>(0))?;

        let mut paths = Vec::new();
        for r in rows {
            paths.push(r?);
        }
        paths.sort();
        Ok(paths)
    }

    /// Query distinct (lineage_id, qualified_name) pairs touched by a specific commit.
    pub fn query_commit_touched_functions(
        &self,
        commit_hash: &str,
    ) -> rusqlite::Result<Vec<(u64, String)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT DISTINCT lineage_id, qualified_name
            FROM function_touches
            WHERE commit_hash = ?1;",
        )?;

        let rows = stmt.query_map(params![commit_hash], |row| {
            Ok((row.get::<_, u64>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut funcs = Vec::new();
        for r in rows {
            funcs.push(r?);
        }
        funcs.sort_by(|a, b| a.1.cmp(&b.1));
        Ok(funcs)
    }

    /// Query commits that touched a specific file path, returning (hash, authored_at, message).
    ///
    /// Used by `ImpactAnalyzer` for follow-up fix counting (GRAPH-03).
    pub fn query_commits_touching_path(
        &self,
        path: &str,
    ) -> rusqlite::Result<Vec<(String, i64, String)>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT DISTINCT c.hash, c.authored_at, c.message
            FROM commits c
            JOIN function_touches ft ON c.hash = ft.commit_hash
            WHERE ft.path = ?1
            ORDER BY c.topo_order ASC;",
        )?;

        let rows = stmt.query_map(params![path], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }
}
