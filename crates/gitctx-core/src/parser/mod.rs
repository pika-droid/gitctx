pub mod behavior;
pub mod cache;
pub mod degrade;
pub mod languages;
pub mod registry;

pub use behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
pub use cache::{BlobParseCache, DEFAULT_BLOB_CACHE_CAPACITY};
pub use degrade::{DegradationHandler, ParsedFileSymbols};
pub use registry::LanguageRegistry;

use crate::error::{CoreError, Result};
use crate::model::ParseStatus;
use git2::Oid;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tree_sitter::Parser;

/// Telemetry metrics for AST parsing operations.
#[derive(Debug, Default)]
pub struct ParseTelemetry {
    pub ok_count: AtomicU64,
    pub degraded_count: AtomicU64,
    pub failed_count: AtomicU64,
}

impl ParseTelemetry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_ok(&self) {
        self.ok_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_degraded(&self) {
        self.degraded_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failed(&self) {
        self.failed_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.ok_count.load(Ordering::Relaxed),
            self.degraded_count.load(Ordering::Relaxed),
            self.failed_count.load(Ordering::Relaxed),
        )
    }
}

/// Coordinated AST extractor with language detection, OID cache, and error degradation.
pub struct AstExtractor {
    registry: LanguageRegistry,
    cache: BlobParseCache,
    telemetry: Arc<ParseTelemetry>,
}

impl Default for AstExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl AstExtractor {
    /// Create a new AST extractor with default registry and cache.
    pub fn new() -> Self {
        Self {
            registry: LanguageRegistry::new(),
            cache: BlobParseCache::default(),
            telemetry: Arc::new(ParseTelemetry::new()),
        }
    }

    /// Access the telemetry tracker.
    pub fn telemetry(&self) -> &Arc<ParseTelemetry> {
        &self.telemetry
    }

    /// Access the blob cache.
    pub fn cache(&self) -> &BlobParseCache {
        &self.cache
    }

    /// Access the language registry.
    pub fn registry(&self) -> &LanguageRegistry {
        &self.registry
    }

    /// Parse a historical blob given its path, raw bytes, and optional git OID.
    pub fn parse_blob(
        &self,
        path: &Path,
        content: &[u8],
        oid: Option<Oid>,
    ) -> Result<ParsedFileSymbols> {
        let Some(behavior) = self.registry.detect(path, Some(content)) else {
            return Ok(ParsedFileSymbols {
                symbols: Vec::new(),
                status: ParseStatus::Ok,
                error_byte_ranges: Vec::new(),
            });
        };

        let lang_id = behavior.language_id();

        // Check cache if OID is provided
        if let Some(blob_oid) = oid {
            if let Some(cached_symbols) = self.cache.get(blob_oid, lang_id) {
                self.telemetry.record_ok();
                return Ok(ParsedFileSymbols {
                    symbols: cached_symbols,
                    status: ParseStatus::Ok,
                    error_byte_ranges: Vec::new(),
                });
            }
        }

        let mut parser = Parser::new();
        if let Err(e) = parser.set_language(&behavior.tree_sitter_language()) {
            self.telemetry.record_failed();
            return Err(CoreError::ParseError(format!(
                "Failed to set tree-sitter language for {}: {e}",
                lang_id.as_str()
            )));
        }

        let Some(tree) = parser.parse(content, None) else {
            self.telemetry.record_failed();
            return Err(CoreError::ParseError(format!(
                "Tree-sitter failed to produce AST for {}",
                path.display()
            )));
        };

        let (has_error, error_ranges) = DegradationHandler::analyze(&tree);
        let extracted_symbols = behavior.extract_symbols(content, &tree)?;

        let (final_symbols, status) = if has_error {
            self.telemetry.record_degraded();
            let mut symbols = extracted_symbols;
            let line_count = u32::try_from(content.split(|&b| b == b'\n').count()).unwrap_or(1);
            symbols.push(DegradationHandler::create_file_level_symbol(line_count, content.len()));
            (symbols, ParseStatus::Degraded)
        } else {
            self.telemetry.record_ok();
            (extracted_symbols, ParseStatus::Ok)
        };

        if let Some(blob_oid) = oid {
            if status == ParseStatus::Ok {
                self.cache.put(blob_oid, lang_id, final_symbols.clone());
            }
        }

        Ok(ParsedFileSymbols {
            symbols: final_symbols,
            status,
            error_byte_ranges: error_ranges,
        })
    }
}
