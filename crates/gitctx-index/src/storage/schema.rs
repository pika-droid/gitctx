use rusqlite::{Connection, Result};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

pub const SCHEMA_DDL: &str = r"
CREATE TABLE IF NOT EXISTS index_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commits (
    hash TEXT PRIMARY KEY,
    author TEXT NOT NULL,
    email TEXT NOT NULL,
    authored_at INTEGER NOT NULL,
    topo_order INTEGER NOT NULL,
    branch TEXT,
    parent_hashes TEXT NOT NULL,
    message TEXT NOT NULL,
    insertions INTEGER NOT NULL,
    deletions INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_commits_topo_order ON commits(topo_order ASC);

CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    language TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_files_path ON files(path);

CREATE TABLE IF NOT EXISTS function_lineage (
    lineage_id INTEGER PRIMARY KEY,
    current_path TEXT NOT NULL,
    current_qualified_name TEXT NOT NULL,
    language TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_lineage_qualname ON function_lineage(current_qualified_name);

CREATE TABLE IF NOT EXISTS function_touches (
    touch_id INTEGER PRIMARY KEY,
    lineage_id INTEGER NOT NULL REFERENCES function_lineage(lineage_id),
    commit_hash TEXT NOT NULL REFERENCES commits(hash),
    path TEXT NOT NULL,
    qualified_name TEXT NOT NULL,
    arity INTEGER NOT NULL,
    change_type TEXT NOT NULL,
    signature_before TEXT,
    signature_after TEXT,
    fingerprint BLOB,
    chunk_text TEXT NOT NULL,
    parse_status TEXT NOT NULL,
    start_line INTEGER NOT NULL DEFAULT 0,
    end_line INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS ix_touch_lineage ON function_touches(lineage_id, commit_hash);
CREATE INDEX IF NOT EXISTS ix_touch_qualname ON function_touches(qualified_name);
CREATE INDEX IF NOT EXISTS ix_touch_commit ON function_touches(commit_hash);
CREATE INDEX IF NOT EXISTS ix_touch_path ON function_touches(path);

CREATE TABLE IF NOT EXISTS embeddings (
    touch_id INTEGER PRIMARY KEY REFERENCES function_touches(touch_id),
    row_offset INTEGER NOT NULL UNIQUE
);
CREATE INDEX IF NOT EXISTS ix_embeddings_row_offset ON embeddings(row_offset);

-- Isolated explain cache (Invariant I5: zero LLM text in retrieval paths)
CREATE TABLE IF NOT EXISTS explain_cache (
    cache_key TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    prompt_version TEXT NOT NULL,
    text TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS cochange_file_metrics (
    entity_a TEXT NOT NULL,
    entity_b TEXT NOT NULL,
    support INTEGER NOT NULL,
    weighted_support REAL NOT NULL,
    confidence_a_to_b REAL NOT NULL,
    confidence_b_to_a REAL NOT NULL,
    lift REAL NOT NULL,
    PRIMARY KEY(entity_a, entity_b)
);
CREATE INDEX IF NOT EXISTS ix_cochange_file_a ON cochange_file_metrics(entity_a);
CREATE INDEX IF NOT EXISTS ix_cochange_file_b ON cochange_file_metrics(entity_b);

CREATE TABLE IF NOT EXISTS cochange_function_metrics (
    entity_a TEXT NOT NULL,
    entity_b TEXT NOT NULL,
    support INTEGER NOT NULL,
    weighted_support REAL NOT NULL,
    confidence_a_to_b REAL NOT NULL,
    confidence_b_to_a REAL NOT NULL,
    lift REAL NOT NULL,
    PRIMARY KEY(entity_a, entity_b)
);
CREATE INDEX IF NOT EXISTS ix_cochange_func_a ON cochange_function_metrics(entity_a);
CREATE INDEX IF NOT EXISTS ix_cochange_func_b ON cochange_function_metrics(entity_b);
";

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA_DDL)?;
    Ok(())
}
