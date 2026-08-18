use rusqlite::{params, Connection};

use crate::storage::writer::ExplainCacheRecord;

/// Isolated explain_cache store API enforcing Invariant I5 (zero LLM-generated text in index/retrieval paths).
pub struct ExplainCacheStore<'a> {
    conn: &'a Connection,
}

impl<'a> ExplainCacheStore<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Retrieve an explanation from the isolated explain_cache by key.
    pub fn get(&self, cache_key: &str) -> rusqlite::Result<Option<ExplainCacheRecord>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT cache_key, model, prompt_version, text, created_at
             FROM explain_cache
             WHERE cache_key = ?1;",
        )?;

        let mut rows = stmt.query(params![cache_key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(ExplainCacheRecord {
                cache_key: row.get(0)?,
                model: row.get(1)?,
                prompt_version: row.get(2)?,
                text: row.get(3)?,
                created_at: row.get(4)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Insert or replace an explanation in the isolated explain_cache.
    pub fn put(
        &self,
        cache_key: &str,
        model: &str,
        prompt_version: &str,
        text: &str,
        created_at: i64,
    ) -> rusqlite::Result<()> {
        let mut stmt = self.conn.prepare_cached(
            "INSERT OR REPLACE INTO explain_cache (cache_key, model, prompt_version, text, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5);",
        )?;
        stmt.execute(params![cache_key, model, prompt_version, text, created_at])?;
        Ok(())
    }

    /// Count total cached explanation records.
    pub fn count(&self) -> rusqlite::Result<usize> {
        self.conn
            .query_row("SELECT COUNT(*) FROM explain_cache;", [], |r| r.get(0))
    }
}
