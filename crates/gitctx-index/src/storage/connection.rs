use rusqlite::{Connection, OpenFlags, Result};
use std::path::Path;

/// Applies performance and consistency PRAGMAs for gitctx SQLite connections.
pub fn apply_pragmas(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5000)?;
    conn.pragma_update(None, "cache_size", -64000)?;
    conn.pragma_update(None, "temp_store", "MEMORY")?;
    conn.pragma_update(None, "mmap_size", 268435456)?;
    Ok(())
}

/// Open a SQLite connection to the given path and apply WAL PRAGMAs.
pub fn open_connection<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_CREATE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    apply_pragmas(&conn)?;
    Ok(conn)
}

/// Open a read-only SQLite connection to the given path and apply WAL PRAGMAs.
pub fn open_readonly_connection<P: AsRef<Path>>(path: P) -> Result<Connection> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    apply_pragmas(&conn)?;
    Ok(conn)
}
