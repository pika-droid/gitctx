use crate::vector::format::{VectorHeader, VECTOR_HEADER_SIZE};
use gitctx_core::error::CoreError;
use memmap2::{Mmap, MmapOptions};
use rusqlite::Connection;
use std::fs::File;
use std::path::Path;

/// Memory-mapped zero-copy dense vector storage reader for `.gitctx/vectors.bin`.
pub struct MmapVectorStorage {
    mmap: Mmap,
    header: VectorHeader,
}

impl MmapVectorStorage {
    /// Open and memory-map a binary vector file from `path`.
    /// Validates magic bytes, format version, dimensions, and file size per D-10 and INDEX-04.
    #[allow(unsafe_code)]
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, CoreError> {
        let p = path.as_ref();
        let file = File::open(p).map_err(|e| CoreError::RepositoryOpen {
            path: p.to_path_buf(),
            message: format!("Failed to open vector file: {e}"),
        })?;

        let metadata = file.metadata().map_err(|e| {
            CoreError::InvariantViolation(format!("Failed to read metadata for vector file: {e}"))
        })?;

        if metadata.len() < VECTOR_HEADER_SIZE as u64 {
            return Err(CoreError::InvariantViolation(format!(
                "Vector file {} is too small ({} bytes, min header is 16 bytes)",
                p.display(),
                metadata.len()
            )));
        }

        let mmap = unsafe {
            MmapOptions::new().map(&file).map_err(|e| {
                CoreError::InvariantViolation(format!(
                    "Failed to memory-map vector file {}: {e}",
                    p.display()
                ))
            })?
        };

        let header = VectorHeader::from_bytes(&mmap[0..VECTOR_HEADER_SIZE])?;

        let expected_size = header.expected_file_size();
        if mmap.len() != expected_size {
            return Err(CoreError::InvariantViolation(format!(
                "Vector file size mismatch for {}: expected {} bytes for {} vectors of dim {}, got {} bytes",
                p.display(),
                expected_size,
                header.count,
                header.dimension,
                mmap.len()
            )));
        }

        Ok(Self { mmap, header })
    }

    /// Return parsed vector header.
    pub fn header(&self) -> &VectorHeader {
        &self.header
    }

    /// Total number of stored dense vectors.
    pub fn count(&self) -> usize {
        self.header.count as usize
    }

    /// Dimension of stored dense vectors (e.g. 384).
    pub fn dimension(&self) -> usize {
        self.header.dimension as usize
    }

    /// Return all stored float elements as a contiguous `&[f32]` slice.
    ///
    /// # Safety
    /// The `VECTOR_HEADER_SIZE` is statically asserted to be 4-byte aligned, and a runtime
    /// debug assertion verifies pointer alignment before the cast. This guarantees the
    /// reinterpretation from `&[u8]` to `&[f32]` is sound on all architectures (including ARM/RISC-V).
    #[allow(unsafe_code)]
    pub fn all_vectors(&self) -> &[f32] {
        // Compile-time guarantee: header size is a multiple of f32 alignment (4 bytes).
        const _: () = assert!(
            VECTOR_HEADER_SIZE % std::mem::align_of::<f32>() == 0,
            "VECTOR_HEADER_SIZE must be aligned to f32 (4 bytes)"
        );

        let count = self.count();
        let dim = self.dimension();
        let total_elements = count * dim;
        if total_elements == 0 {
            return &[];
        }

        let payload_ptr = unsafe { self.mmap.as_ptr().add(VECTOR_HEADER_SIZE) };
        debug_assert_eq!(
            payload_ptr as usize % std::mem::align_of::<f32>(),
            0,
            "mmap payload pointer is not 4-byte aligned for f32 access"
        );
        let payload_ptr = payload_ptr.cast::<f32>();
        unsafe { std::slice::from_raw_parts(payload_ptr, total_elements) }
    }

    /// Retrieve zero-copy slice for vector at `row_offset`.
    pub fn get_vector(&self, row_offset: usize) -> Option<&[f32]> {
        if row_offset >= self.count() {
            return None;
        }

        let dim = self.dimension();
        let start = row_offset * dim;
        let end = start + dim;
        let all = self.all_vectors();
        Some(&all[start..end])
    }

    /// Validate vector storage count against SQLite `embeddings` table row count.
    pub fn validate_with_sqlite(&self, conn: &Connection) -> Result<(), CoreError> {
        let sqlite_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM embeddings;", [], |row| row.get(0))
            .map_err(|e| {
                CoreError::InvariantViolation(format!("Failed to query embeddings count: {e}"))
            })?;

        if sqlite_count as usize != self.count() {
            return Err(CoreError::InvariantViolation(format!(
                "Vector count mismatch: vectors.bin has {} vectors, but SQLite embeddings table has {} rows",
                self.count(),
                sqlite_count
            )));
        }

        Ok(())
    }
}
