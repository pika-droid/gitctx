use gitctx_core::error::CoreError;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

pub const VECTOR_MAGIC: &[u8; 4] = b"GVEC";
pub const VECTOR_VERSION: u32 = 1;
pub const VECTOR_DIMENSION: u32 = 384;
pub const VECTOR_HEADER_SIZE: usize = 16;

/// 16-byte fixed binary header for `.gitctx/vectors.bin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub dimension: u32,
    pub count: u32,
}

impl VectorHeader {
    /// Create a new VectorHeader with current version and specified count & dimension.
    pub fn new(count: u32, dimension: u32) -> Self {
        Self {
            magic: *VECTOR_MAGIC,
            version: VECTOR_VERSION,
            dimension,
            count,
        }
    }

    /// Serialize header to 16 little-endian bytes.
    pub fn to_bytes(&self) -> [u8; VECTOR_HEADER_SIZE] {
        let mut bytes = [0u8; VECTOR_HEADER_SIZE];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4..8].copy_from_slice(&self.version.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.dimension.to_le_bytes());
        bytes[12..16].copy_from_slice(&self.count.to_le_bytes());
        bytes
    }

    /// Parse header from 16 bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() < VECTOR_HEADER_SIZE {
            return Err(CoreError::InvariantViolation(
                "Vector file header too short: expected at least 16 bytes".to_string(),
            ));
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);
        if &magic != VECTOR_MAGIC {
            return Err(CoreError::InvariantViolation(format!(
                "Invalid vector file magic: expected {VECTOR_MAGIC:?}, got {magic:?}"
            )));
        }

        let version = u32::from_le_bytes(bytes[4..8].try_into().map_err(|_| {
            CoreError::InvariantViolation("Failed to parse vector file version".to_string())
        })?);
        if version != VECTOR_VERSION {
            return Err(CoreError::InvariantViolation(format!(
                "Unsupported vector file version: expected {VECTOR_VERSION}, got {version}"
            )));
        }

        let dimension = u32::from_le_bytes(bytes[8..12].try_into().map_err(|_| {
            CoreError::InvariantViolation("Failed to parse vector dimension".to_string())
        })?);

        let count = u32::from_le_bytes(bytes[12..16].try_into().map_err(|_| {
            CoreError::InvariantViolation("Failed to parse vector count".to_string())
        })?);

        Ok(Self {
            magic,
            version,
            dimension,
            count,
        })
    }

    /// Calculate expected total file size in bytes including the 16-byte header.
    pub fn expected_file_size(&self) -> usize {
        VECTOR_HEADER_SIZE
            + (self.count as usize * self.dimension as usize * std::mem::size_of::<f32>())
    }
}

/// Write vectors to a binary file at `path`.
pub fn write_vector_file<P: AsRef<Path>>(
    path: P,
    vectors: &[f32],
    dimension: u32,
    count: u32,
) -> Result<(), CoreError> {
    let expected_floats = count as usize * dimension as usize;
    if vectors.len() != expected_floats {
        return Err(CoreError::InvariantViolation(format!(
            "Vector slice length mismatch: expected {} f32 elements ({} vectors * {} dim), got {}",
            expected_floats,
            count,
            dimension,
            vectors.len()
        )));
    }

    let header = VectorHeader::new(count, dimension);
    let file =
        File::create(path.as_ref()).map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
    let mut writer = BufWriter::new(file);

    writer
        .write_all(&header.to_bytes())
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;

    for val in vectors {
        writer
            .write_all(&val.to_le_bytes())
            .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
    }

    writer
        .flush()
        .map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
    Ok(())
}

/// Write vectors atomically to `.gitctx/vectors.bin` via temporary file replacement.
pub fn write_vector_file_atomic<P: AsRef<Path>>(
    target_path: P,
    vectors: &[f32],
    dimension: u32,
    count: u32,
) -> Result<(), CoreError> {
    let target = target_path.as_ref();
    let tmp_path = target.with_extension("bin.tmp");

    write_vector_file(&tmp_path, vectors, dimension, count)?;

    std::fs::rename(&tmp_path, target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp_path);
        CoreError::InvariantViolation(format!(
            "Failed atomic vector file rename from {} to {}: {e}",
            tmp_path.display(),
            target.display()
        ))
    })?;

    Ok(())
}

/// Read header from a vector file on disk.
pub fn read_vector_header<P: AsRef<Path>>(path: P) -> Result<VectorHeader, CoreError> {
    let mut file =
        File::open(path.as_ref()).map_err(|e| CoreError::InvariantViolation(e.to_string()))?;
    let mut buf = [0u8; VECTOR_HEADER_SIZE];
    file.read_exact(&mut buf)
        .map_err(|e| CoreError::InvariantViolation(format!("Failed to read vector header: {e}")))?;
    VectorHeader::from_bytes(&buf)
}
