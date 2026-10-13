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
        let base = db_buf
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let tmp_db_name = format!(
            "{}.tmp",
            db_buf
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("index.db")
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

    /// Perform two-phase staged atomic swap across SQLite database, Tantivy directory, and vectors file.
    ///
    /// Phase 1 stages all existing live artifacts to `.bak`.
    /// Phase 2 promotes new `.tmp` artifacts to active paths.
    /// If any promotion fails, all active artifacts are restored from `.bak` preserving full consistency.
    /// Upon success, all `.bak` staging artifacts and stale WAL/SHM files are purged.
    pub fn commit_swap(&mut self) -> std::io::Result<()> {
        if let Some(parent) = self.db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let bak_db = self.db_path.with_extension("bak");
        let bak_tantivy = self.tantivy_dir.with_extension("bak");
        let bak_vectors = self.vectors_path.with_extension("bak");

        let remove_bak = |bak: &Path| {
            if bak.is_dir() {
                let _ = fs::remove_dir_all(bak);
            } else if bak.exists() {
                let _ = fs::remove_file(bak);
            }
        };

        // Phase 1: Stage existing active resources to .bak
        let mut db_backed_up = false;
        let mut tantivy_backed_up = false;
        let mut vectors_backed_up = false;

        let mut stage_backups = || -> std::io::Result<()> {
            if self.db_path.exists() {
                remove_bak(&bak_db);
                fs::rename(&self.db_path, &bak_db)?;
                db_backed_up = true;
            }
            if self.tantivy_dir.exists() {
                remove_bak(&bak_tantivy);
                fs::rename(&self.tantivy_dir, &bak_tantivy)?;
                tantivy_backed_up = true;
            }
            if self.vectors_path.exists() {
                remove_bak(&bak_vectors);
                fs::rename(&self.vectors_path, &bak_vectors)?;
                vectors_backed_up = true;
            }
            Ok(())
        };

        if let Err(e) = stage_backups() {
            if vectors_backed_up && bak_vectors.exists() {
                let _ = fs::rename(&bak_vectors, &self.vectors_path);
            }
            if tantivy_backed_up && bak_tantivy.exists() {
                let _ = fs::rename(&bak_tantivy, &self.tantivy_dir);
            }
            if db_backed_up && bak_db.exists() {
                let _ = fs::rename(&bak_db, &self.db_path);
            }
            return Err(e);
        }

        // Phase 2: Promote tmp resources to active locations
        let mut db_promoted = false;
        let mut tantivy_promoted = false;
        let mut vectors_promoted = false;

        let mut promote_tmp = || -> std::io::Result<()> {
            if self.tmp_db_path.exists() {
                fs::rename(&self.tmp_db_path, &self.db_path)?;
                db_promoted = true;
            }
            if self.tmp_tantivy_dir.exists() {
                fs::rename(&self.tmp_tantivy_dir, &self.tantivy_dir)?;
                tantivy_promoted = true;
            }
            if self.tmp_vectors_path.exists() {
                fs::rename(&self.tmp_vectors_path, &self.vectors_path)?;
                vectors_promoted = true;
            }
            Ok(())
        };

        if let Err(e) = promote_tmp() {
            // Full rollback: revert promoted tmp files and restore active from .bak
            if vectors_promoted && self.vectors_path.exists() {
                let _ = fs::rename(&self.vectors_path, &self.tmp_vectors_path);
            }
            if tantivy_promoted && self.tantivy_dir.exists() {
                let _ = fs::rename(&self.tantivy_dir, &self.tmp_tantivy_dir);
            }
            if db_promoted && self.db_path.exists() {
                let _ = fs::rename(&self.db_path, &self.tmp_db_path);
            }

            if vectors_backed_up && bak_vectors.exists() {
                let _ = fs::rename(&bak_vectors, &self.vectors_path);
            }
            if tantivy_backed_up && bak_tantivy.exists() {
                let _ = fs::rename(&bak_tantivy, &self.tantivy_dir);
            }
            if db_backed_up && bak_db.exists() {
                let _ = fs::rename(&bak_db, &self.db_path);
            }
            return Err(e);
        }

        // Phase 3: Purge .bak backups and cleanup any stale WAL/SHM files
        remove_bak(&bak_db);
        remove_bak(&bak_tantivy);
        remove_bak(&bak_vectors);

        let clean_wal_shm = |path: &Path| {
            let wal = std::path::PathBuf::from(format!("{}-wal", path.display()));
            let shm = std::path::PathBuf::from(format!("{}-shm", path.display()));
            let _ = fs::remove_file(wal);
            let _ = fs::remove_file(shm);
        };
        clean_wal_shm(&self.tmp_db_path);
        clean_wal_shm(&self.db_path);

        self.committed = true;
        Ok(())
    }

    /// Clean up any temporary `.tmp` and `.bak` files.
    pub fn cleanup(&self) {
        if self.tmp_db_path.exists() {
            let _ = fs::remove_file(&self.tmp_db_path);
        }
        let clean_wal_shm = |path: &Path| {
            let wal1 = path.with_extension("tmp-wal");
            let shm1 = path.with_extension("tmp-shm");
            let wal2 = std::path::PathBuf::from(format!("{}-wal", path.display()));
            let shm2 = std::path::PathBuf::from(format!("{}-shm", path.display()));
            let _ = fs::remove_file(wal1);
            let _ = fs::remove_file(shm1);
            let _ = fs::remove_file(wal2);
            let _ = fs::remove_file(shm2);
        };
        clean_wal_shm(&self.tmp_db_path);

        let bak_db = self.db_path.with_extension("bak");
        if bak_db.exists() {
            let _ = fs::remove_file(bak_db);
        }

        if self.tmp_tantivy_dir.exists() {
            let _ = fs::remove_dir_all(&self.tmp_tantivy_dir);
        }
        let bak_tantivy = self.tantivy_dir.with_extension("bak");
        if bak_tantivy.exists() {
            let _ = fs::remove_dir_all(bak_tantivy);
        }

        if self.tmp_vectors_path.exists() {
            let _ = fs::remove_file(&self.tmp_vectors_path);
        }
        let bak_vec = self.vectors_path.with_extension("bak");
        if bak_vec.exists() {
            let _ = fs::remove_file(bak_vec);
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
