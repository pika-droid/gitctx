use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use gitctx_core::error::CoreError;
use gitctx_core::model::ParseStatus;

pub const DEFAULT_EMBEDDING_MODEL: EmbeddingModel = EmbeddingModel::BGESmallENV15;
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 384;
pub const DEFAULT_BATCH_SIZE: usize = 128;

/// Input item for batch embedding.
#[derive(Debug, Clone)]
pub struct EmbeddingInput {
    pub touch_id: u64,
    pub text: String,
    pub parse_status: ParseStatus,
}

/// Output item with touch_id and computed embedding vector.
#[derive(Debug, Clone)]
pub struct EmbeddingOutput {
    pub touch_id: u64,
    pub embedding: Vec<f32>,
}

/// Offline CPU embedding generator powered by fastembed-rs.
pub struct FastEmbedGenerator {
    model: TextEmbedding,
    dimension: usize,
}

impl FastEmbedGenerator {
    /// Initialize with default or specified embedding model.
    pub fn new(model: Option<EmbeddingModel>) -> Result<Self, CoreError> {
        let model_type = model.unwrap_or(DEFAULT_EMBEDDING_MODEL);
        let options = InitOptions::new(model_type).with_show_download_progress(false);
        let model = TextEmbedding::try_new(options)
            .map_err(|e| CoreError::InvariantViolation(format!("Failed to initialize FastEmbed model: {e}")))?;

        Ok(Self {
            model,
            dimension: DEFAULT_EMBEDDING_DIMENSION,
        })
    }

    /// Return expected embedding dimension (384 for BGESmallENV15 / AllMiniLML6V2).
    pub fn dimension(&self) -> usize {
        self.dimension
    }

    /// Embed a slice of text strings with optional batch size.
    pub fn embed_texts<S: AsRef<str>>(&mut self, texts: &[S], batch_size: Option<usize>) -> Result<Vec<Vec<f32>>, CoreError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_ref()).collect();
        let batch = batch_size.unwrap_or(DEFAULT_BATCH_SIZE);
        self.model
            .embed(text_refs, Some(batch))
            .map_err(|e| CoreError::InvariantViolation(format!("Embedding generation failed: {e}")))
    }

    /// Generate embeddings for a collection of chunk inputs.
    /// Filters chunks according to D-12: embeds `ParseStatus::Ok` and `ParseStatus::Degraded`, skips `ParseStatus::Failed`.
    /// Embeddings are produced in batches of `batch_size` (defaults to 128..256).
    pub fn generate_embeddings_batched(
        &mut self,
        chunks: &[EmbeddingInput],
        batch_size: usize,
    ) -> Result<Vec<EmbeddingOutput>, CoreError> {
        if chunks.is_empty() {
            return Ok(Vec::new());
        }

        // Filter chunks: embed Ok and Degraded, skip Failed (D-12)
        let eligible: Vec<&EmbeddingInput> = chunks
            .iter()
            .filter(|c| matches!(c.parse_status, ParseStatus::Ok | ParseStatus::Degraded))
            .collect();

        if eligible.is_empty() {
            return Ok(Vec::new());
        }

        let texts: Vec<&str> = eligible.iter().map(|c| c.text.as_str()).collect();
        let effective_batch = if batch_size == 0 { DEFAULT_BATCH_SIZE } else { batch_size };
        let raw_embeddings = self
            .model
            .embed(texts, Some(effective_batch))
            .map_err(|e| CoreError::InvariantViolation(format!("Batched embedding generation failed: {e}")))?;

        if raw_embeddings.len() != eligible.len() {
            return Err(CoreError::InvariantViolation(format!(
                "Embedding generator returned {} vectors for {} inputs",
                raw_embeddings.len(),
                eligible.len()
            )));
        }

        let results = eligible
            .iter()
            .zip(raw_embeddings)
            .map(|(input, emb)| EmbeddingOutput {
                touch_id: input.touch_id,
                embedding: emb,
            })
            .collect();

        Ok(results)
    }
}
