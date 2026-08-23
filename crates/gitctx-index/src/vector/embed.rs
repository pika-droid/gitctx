use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use gitctx_core::error::CoreError;
use gitctx_core::model::ParseStatus;
use std::path::PathBuf;
use std::sync::Once;

pub const DEFAULT_EMBEDDING_MODEL: EmbeddingModel = EmbeddingModel::BGESmallENV15;
pub const DEFAULT_EMBEDDING_DIMENSION: usize = 384;
pub const DEFAULT_BATCH_SIZE: usize = 128;

static ORT_INIT_ONCE: Once = Once::new();

fn try_find_onnxruntime_dylib() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let lib_name = "onnxruntime.dll";
    #[cfg(target_os = "macos")]
    let lib_name = "libonnxruntime.dylib";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let lib_name = "libonnxruntime.so";

    // 1. Explicit env var
    if let Ok(path_str) = std::env::var("ORT_DYLIB_PATH") {
        let p = PathBuf::from(path_str);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Next to executable or in deps folder
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let direct = parent.join(lib_name);
            if direct.exists() {
                return Some(direct);
            }
            let deps = parent.join("deps").join(lib_name);
            if deps.exists() {
                return Some(deps);
            }
            if let Some(grandparent) = parent.parent() {
                let gp = grandparent.join(lib_name);
                if gp.exists() {
                    return Some(gp);
                }
                let gp_deps = grandparent.join("deps").join(lib_name);
                if gp_deps.exists() {
                    return Some(gp_deps);
                }
            }
        }
    }

    // 3. Current working directory or target directory
    if let Ok(cwd) = std::env::current_dir() {
        let candidates = [
            cwd.join(lib_name),
            cwd.join("target").join("debug").join(lib_name),
            cwd.join("target").join("debug").join("deps").join(lib_name),
            cwd.join("target").join("release").join(lib_name),
            cwd.join("target")
                .join("release")
                .join("deps")
                .join(lib_name),
        ];
        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate);
            }
        }
    }

    // 4. Cache directory from build script
    let temp = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&temp) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("gitctx-ort-") {
                let candidate = entry.path().join(lib_name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

/// Ensure ONNX Runtime dynamic library is initialized before any FastEmbed call.
fn ensure_ort_initialized() {
    ORT_INIT_ONCE.call_once(|| {
        if let Some(dylib_path) = try_find_onnxruntime_dylib() {
            let _ = ort::init_from(dylib_path.to_string_lossy()).commit();
        }
    });
}

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
        ensure_ort_initialized();

        let model_type = model.unwrap_or(DEFAULT_EMBEDDING_MODEL);
        let options = InitOptions::new(model_type).with_show_download_progress(false);

        let model_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            TextEmbedding::try_new(options)
        }));

        let model = match model_res {
            Ok(Ok(m)) => m,
            Ok(Err(e)) => {
                return Err(CoreError::InvariantViolation(format!(
                    "Failed to initialize FastEmbed model: {e}"
                )));
            }
            Err(_) => {
                return Err(CoreError::InvariantViolation(
                    "Panic occurred while initializing ONNX Runtime / FastEmbed model".to_string(),
                ));
            }
        };

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
    pub fn embed_texts<S: AsRef<str>>(
        &mut self,
        texts: &[S],
        batch_size: Option<usize>,
    ) -> Result<Vec<Vec<f32>>, CoreError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let text_refs: Vec<&str> = texts.iter().map(AsRef::as_ref).collect();
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
        let effective_batch = if batch_size == 0 {
            DEFAULT_BATCH_SIZE
        } else {
            batch_size
        };
        let raw_embeddings = self
            .model
            .embed(texts, Some(effective_batch))
            .map_err(|e| {
                CoreError::InvariantViolation(format!("Batched embedding generation failed: {e}"))
            })?;

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
