pub mod embed;
pub mod format;
pub mod mmap;
pub mod simd;

pub use embed::{
    EmbeddingInput, EmbeddingOutput, FastEmbedGenerator, DEFAULT_BATCH_SIZE,
    DEFAULT_EMBEDDING_DIMENSION, DEFAULT_EMBEDDING_MODEL,
};
pub use format::{
    read_vector_header, write_vector_file, write_vector_file_atomic, VectorHeader,
    VECTOR_DIMENSION, VECTOR_HEADER_SIZE, VECTOR_MAGIC, VECTOR_VERSION,
};
pub use mmap::MmapVectorStorage;
pub use simd::{dot_product_simd, normalize_l2, normalized_l2, search_cosine_top_k};
