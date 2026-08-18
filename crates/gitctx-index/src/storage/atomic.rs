use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

/// Manages crash-safe two-phase atomic database and index file swaps per D-02 and INDEX-01.
#[derive(Debug)]
pub struct AtomicStorageManager {
    base_dir: PathBuf,
    db_path: PathBuf,
    tmp_db_path: PathBuf,
    tantivy_dir: PathBuf,
    tmp_tantivy_dir: PathBuf,
    vectors_path: PathBuf,
    tmp_vectors_path: PathBuf,
    committed: bool,
}

impl AtomicStorageManager {
    /// Create manager for standard `.gitctx` index directory.
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        let base = base_dir.as_ref().to_path_buf();
        Self {
            db_path: base.join("index.db"),
            tmp_db_path: base.join("index.db.tmp"),
            tantivy_dir: base.join("tantivy"),
            tmp_tantivy_dir: base.join("tantivy.tmp"),
            vectors_path: base.join("vectors.bin"),
            tmp_vectors_path: base.join("vectors.bin.tmp"),
            base_dir: base,
            committed: false,
        }
    }

    /// Create manager targeting a specific database path.
    pub fn for_db_file<P: AsRef<Path>>(db_path: P) -> Self {
        let db_buf = db_path.as_ref().to_path_buf();
        let base = db_buf.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
        let tmp_db_name = format!(
            "{}.tmp",
            db_buf.file_name().and_then(|n| n.to_str()).unwrap_or("index.db")
        );
        let tmp_db = base.join(tmp_db_name);
        Self {
            db_path: db_buf,
            tmp_db_path: tmp_db,
            tantivy_dir: base.join("tantivy"),
            tmp_tantivy_dir: base.join("tantivy.tmp"),
            vectors_path: base.join("vectors.bin"),
            tmp_vectors_path: base.join("vectors.bin.tmp"),
            base_dir: base,
            committed: false,
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn tmp_db_path(&self) -> &Path {
        &self.tmp_db_path
    }

    pub fn tantivy_dir(&self) -> &Path {
        &self.tantivy_dir
    }

    pub fn tmp_tantivy_dir(&self) -> &Path {
        &self.tmp_tantivy_dir
    }

    pub fn vectors_path(&self) -> &Path {
        &self.vectors_path
    }

    pub fn tmp_vectors_path(&self) -> &Path {
        &self.tmp_vectors_path
    }

    /// Checkpoint WAL on SQLite connection and close connection before swapping files.
    pub fn checkpoint_and_close(conn: Connection) -> rusqlite::Result<()> {
        conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
        conn.close().map_err(|(_, e)| e)?;
        Ok(())
    }

    /// Safely replace target with source, handling Windows atomic rename constraints.
    fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
        if !source.exists() {
            return Ok(());
        }
        if target.exists() {
            fs::remove_file(target)?;
        }
        fs::rename(source, target)?;
        Ok(())
    }

    /// Safely replace target directory with source directory.
    fn replace_dir(source: &Path, target: &Path) -> std::io::Result<()> {
        if !source.exists() {
            return Ok(());
        }
        if target.exists() {
            fs::remove_dir_all(target)?;
        }
        fs::rename(source, target)?;
        Ok(())
    }

    /// Perform atomic swap across SQLite database, Tantivy directory, and vectors file.
    pub fn commit_swap(&mut self) -> std::io::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 1. Swap SQLite DB
        Self::replace_file(&self.tmp_db_path, &self.db_path)?;

        // Remove temporary WAL/SHM if left over
        let wal_tmp = self.tmp_db_path.with_extension("tmp-wal");
        let shm_tmp = self.tmp_db_path.with_extension("tmp-shm");
        let _ = fs::remove_file(wal_tmp);
        let _ = fs::remove_file(shm_tmp);

        // 2. Swap Tantivy index directory if exists
        Self::replace_dir(&self.tmp_tantivy_dir, &self.tantivy_dir)?;

        // 3. Swap vectors binary file if exists
        Self::replace_file(&self.tmp_vectors_path, &self.vectors_path)?;

        self.committed = true;
        Ok(())
    }

    /// Clean up any temporary `.tmp` files.
    pub fn cleanup(&self) {
        if self.tmp_db_path.exists() {
            let _ = fs::remove_file(&self.tmp_db_path);
        }
        let wal_tmp = self.tmp_db_path.with_extension("tmp-wal");
        let shm_tmp = self.tmp_db_path.with_extension("tmp-shm");
        let _ = fs::remove_file(wal_tmp);
        let _ = fs::remove_file(shm_tmp);

        if self.tmp_tantivy_dir.exists() {
            let _ = fs::remove_dir_all(&self.tmp_tantivy_dir);
        }
        if self.tmp_vectors_path.exists() {
            let _ = fs::remove_file(&self.tmp_vectors_path);
        }
    }
}

impl Drop for AtomicStorageManager {
    fn drop(&mut self) {
        if !self.committed {
            self.cleanup();
        }
    }
}
