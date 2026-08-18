use crossbeam_channel::{bounded, Sender};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::thread::{self, JoinHandle};

use crate::storage::schema::initialize_schema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitRecord {
    pub hash: String,
    pub author: String,
    pub email: String,
    pub authored_at: i64,
    pub topo_order: u32,
    pub branch: Option<String>,
    pub parent_hashes: String,
    pub message: String,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: Option<i64>,
    pub path: String,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageRecord {
    pub lineage_id: u64,
    pub current_path: String,
    pub current_qualified_name: String,
    pub language: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TouchRecord {
    pub touch_id: Option<i64>,
    pub lineage_id: u64,
    pub commit_hash: String,
    pub path: String,
    pub qualified_name: String,
    pub arity: u32,
    pub change_type: String,
    pub signature_before: Option<String>,
    pub signature_after: Option<String>,
    pub fingerprint: Option<Vec<u8>>,
    pub chunk_text: String,
    pub parse_status: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingRecord {
    pub touch_id: i64,
    pub row_offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplainCacheRecord {
    pub cache_key: String,
    pub model: String,
    pub prompt_version: String,
    pub text: String,
    pub created_at: i64,
}

#[derive(Debug)]
pub enum WriteMessage {
    InsertCommits(Vec<CommitRecord>),
    InsertFiles(Vec<FileRecord>),
    InsertLineages(Vec<LineageRecord>),
    InsertTouches(Vec<TouchRecord>),
    InsertEmbeddings(Vec<EmbeddingRecord>),
    SetIndexState { key: String, value: String },
    Flush(Sender<Result<(), String>>),
    Close(Sender<Result<(), String>>),
}

pub struct BackgroundWriter {
    sender: Sender<WriteMessage>,
    handle: Option<JoinHandle<Result<Connection, String>>>,
}

impl BackgroundWriter {
    pub fn new(conn: Connection, batch_size: usize) -> Self {
        let (sender, receiver) = bounded::<WriteMessage>(2048);

        let handle = thread::spawn(move || {
            // Initialize schema if not already present
            if let Err(e) = initialize_schema(&conn) {
                return Err(format!("Failed to initialize schema: {e}"));
            }

            let mut in_txn = false;
            let mut uncommitted_count = 0usize;

            let begin_txn = |in_txn: &mut bool| -> Result<(), String> {
                if !*in_txn {
                    conn.execute_batch("BEGIN IMMEDIATE;")
                        .map_err(|e| format!("Failed to begin transaction: {e}"))?;
                    *in_txn = true;
                }
                Ok(())
            };

            let commit_txn = |in_txn: &mut bool, uncommitted: &mut usize| -> Result<(), String> {
                if *in_txn {
                    conn.execute_batch("COMMIT;")
                        .map_err(|e| format!("Failed to commit transaction: {e}"))?;
                    *in_txn = false;
                    *uncommitted = 0;
                }
                Ok(())
            };

            while let Ok(msg) = receiver.recv() {
                match msg {
                    WriteMessage::InsertCommits(commits) => {
                        begin_txn(&mut in_txn)?;
                        let mut stmt = conn
                            .prepare_cached(
                                "INSERT OR REPLACE INTO commits (hash, author, email, authored_at, topo_order, branch, parent_hashes, message, insertions, deletions)
                                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10);"
                            )
                            .map_err(|e| format!("Failed to prepare commit statement: {e}"))?;

                        for c in &commits {
                            stmt.execute(params![
                                c.hash,
                                c.author,
                                c.email,
                                c.authored_at,
                                c.topo_order,
                                c.branch,
                                c.parent_hashes,
                                c.message,
                                c.insertions,
                                c.deletions,
                            ])
                            .map_err(|e| format!("Failed to insert commit {}: {e}", c.hash))?;
                        }
                        uncommitted_count += commits.len();
                    }
                    WriteMessage::InsertFiles(files) => {
                        begin_txn(&mut in_txn)?;
                        let mut stmt = conn
                            .prepare_cached(
                                "INSERT OR REPLACE INTO files (id, path, language) VALUES (?1, ?2, ?3);"
                            )
                            .map_err(|e| format!("Failed to prepare file statement: {e}"))?;

                        for f in &files {
                            stmt.execute(params![f.id, f.path, f.language])
                                .map_err(|e| format!("Failed to insert file {}: {e}", f.path))?;
                        }
                        uncommitted_count += files.len();
                    }
                    WriteMessage::InsertLineages(lineages) => {
                        begin_txn(&mut in_txn)?;
                        let mut stmt = conn
                            .prepare_cached(
                                "INSERT OR REPLACE INTO function_lineage (lineage_id, current_path, current_qualified_name, language)
                                 VALUES (?1, ?2, ?3, ?4);"
                            )
                            .map_err(|e| format!("Failed to prepare lineage statement: {e}"))?;

                        for l in &lineages {
                            stmt.execute(params![
                                l.lineage_id,
                                l.current_path,
                                l.current_qualified_name,
                                l.language,
                            ])
                            .map_err(|e| {
                                format!("Failed to insert lineage {}: {e}", l.lineage_id)
                            })?;
                        }
                        uncommitted_count += lineages.len();
                    }
                    WriteMessage::InsertTouches(touches) => {
                        begin_txn(&mut in_txn)?;
                        let mut stmt = conn
                            .prepare_cached(
                                "INSERT INTO function_touches (
                                    touch_id, lineage_id, commit_hash, path, qualified_name, arity, change_type,
                                    signature_before, signature_after, fingerprint, chunk_text, parse_status, start_line, end_line
                                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14);"
                            )
                            .map_err(|e| format!("Failed to prepare touches statement: {e}"))?;

                        for t in &touches {
                            stmt.execute(params![
                                t.touch_id,
                                t.lineage_id,
                                t.commit_hash,
                                t.path,
                                t.qualified_name,
                                t.arity,
                                t.change_type,
                                t.signature_before,
                                t.signature_after,
                                t.fingerprint,
                                t.chunk_text,
                                t.parse_status,
                                i64::try_from(t.start_line).unwrap_or(0),
                                i64::try_from(t.end_line).unwrap_or(0),
                            ])
                            .map_err(|e| {
                                format!("Failed to insert touch for {}: {e}", t.qualified_name)
                            })?;
                        }
                        uncommitted_count += touches.len();
                    }
                    WriteMessage::InsertEmbeddings(embeddings) => {
                        begin_txn(&mut in_txn)?;
                        let mut stmt = conn
                            .prepare_cached(
                                "INSERT OR REPLACE INTO embeddings (touch_id, row_offset) VALUES (?1, ?2);"
                            )
                            .map_err(|e| format!("Failed to prepare embeddings statement: {e}"))?;

                        for emb in &embeddings {
                            stmt.execute(params![emb.touch_id, emb.row_offset])
                                .map_err(|e| {
                                    format!("Failed to insert embedding {}: {e}", emb.touch_id)
                                })?;
                        }
                        uncommitted_count += embeddings.len();
                    }
                    WriteMessage::SetIndexState { key, value } => {
                        begin_txn(&mut in_txn)?;
                        let mut stmt = conn
                            .prepare_cached(
                                "INSERT OR REPLACE INTO index_state (key, value) VALUES (?1, ?2);",
                            )
                            .map_err(|e| format!("Failed to prepare index_state statement: {e}"))?;

                        stmt.execute(params![key, value])
                            .map_err(|e| format!("Failed to set index_state {key}: {e}"))?;
                        uncommitted_count += 1;
                    }
                    WriteMessage::Flush(ack) => {
                        commit_txn(&mut in_txn, &mut uncommitted_count)?;
                        let _ = ack.send(Ok(()));
                    }
                    WriteMessage::Close(ack) => {
                        commit_txn(&mut in_txn, &mut uncommitted_count)?;
                        let _ = ack.send(Ok(()));
                        break;
                    }
                }

                if in_txn && uncommitted_count >= batch_size {
                    commit_txn(&mut in_txn, &mut uncommitted_count)?;
                }
            }

            commit_txn(&mut in_txn, &mut uncommitted_count)?;
            Ok(conn)
        });

        Self {
            sender,
            handle: Some(handle),
        }
    }

    pub fn sender(&self) -> Sender<WriteMessage> {
        self.sender.clone()
    }

    pub fn push_commits(&self, commits: Vec<CommitRecord>) -> Result<(), String> {
        self.sender
            .send(WriteMessage::InsertCommits(commits))
            .map_err(|e| format!("Failed to send InsertCommits: {e}"))
    }

    pub fn push_files(&self, files: Vec<FileRecord>) -> Result<(), String> {
        self.sender
            .send(WriteMessage::InsertFiles(files))
            .map_err(|e| format!("Failed to send InsertFiles: {e}"))
    }

    pub fn push_lineages(&self, lineages: Vec<LineageRecord>) -> Result<(), String> {
        self.sender
            .send(WriteMessage::InsertLineages(lineages))
            .map_err(|e| format!("Failed to send InsertLineages: {e}"))
    }

    pub fn push_touches(&self, touches: Vec<TouchRecord>) -> Result<(), String> {
        self.sender
            .send(WriteMessage::InsertTouches(touches))
            .map_err(|e| format!("Failed to send InsertTouches: {e}"))
    }

    pub fn push_embeddings(&self, embeddings: Vec<EmbeddingRecord>) -> Result<(), String> {
        self.sender
            .send(WriteMessage::InsertEmbeddings(embeddings))
            .map_err(|e| format!("Failed to send InsertEmbeddings: {e}"))
    }

    pub fn set_state(
        &self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), String> {
        self.sender
            .send(WriteMessage::SetIndexState {
                key: key.into(),
                value: value.into(),
            })
            .map_err(|e| format!("Failed to send SetIndexState: {e}"))
    }

    pub fn flush(&self) -> Result<(), String> {
        let (ack_send, ack_recv) = bounded(1);
        self.sender
            .send(WriteMessage::Flush(ack_send))
            .map_err(|e| format!("Failed to send Flush: {e}"))?;
        ack_recv
            .recv()
            .map_err(|e| format!("Failed to receive Flush ack: {e}"))?
    }

    pub fn finish(mut self) -> Result<Connection, String> {
        let (ack_send, ack_recv) = bounded(1);
        let _ = self.sender.send(WriteMessage::Close(ack_send));
        let _ = ack_recv.recv();

        if let Some(handle) = self.handle.take() {
            handle
                .join()
                .map_err(|_| "Background writer thread panicked".to_string())?
        } else {
            Err("Background writer thread already finished".to_string())
        }
    }
}
