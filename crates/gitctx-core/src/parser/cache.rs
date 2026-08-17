use crate::parser::behavior::{LanguageId, RawFunctionSymbol};
use git2::Oid;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Mutex;

/// Default capacity for the in-memory parsed symbol blob LRU cache.
pub const DEFAULT_BLOB_CACHE_CAPACITY: usize = 50_000;

/// Thread-safe LRU cache for parsed historical AST function symbols keyed by `(Oid, LanguageId)`.
pub struct BlobParseCache {
    cache: Mutex<LruCache<(Oid, LanguageId), Vec<RawFunctionSymbol>>>,
}

impl Default for BlobParseCache {
    fn default() -> Self {
        Self::new(DEFAULT_BLOB_CACHE_CAPACITY)
    }
}

impl BlobParseCache {
    /// Create a new blob parse cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1).expect("non-zero"));
        Self {
            cache: Mutex::new(LruCache::new(cap)),
        }
    }

    /// Retrieve cached parsed symbols for `(oid, language_id)` if present.
    pub fn get(&self, oid: Oid, language_id: LanguageId) -> Option<Vec<RawFunctionSymbol>> {
        let mut guard = self.cache.lock().ok()?;
        guard.get(&(oid, language_id)).cloned()
    }

    /// Store parsed symbols for `(oid, language_id)`.
    pub fn put(&self, oid: Oid, language_id: LanguageId, symbols: Vec<RawFunctionSymbol>) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.put((oid, language_id), symbols);
        }
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        if let Ok(mut guard) = self.cache.lock() {
            guard.clear();
        }
    }

    /// Return the number of cached items.
    pub fn len(&self) -> usize {
        self.cache.lock().map_or(0, |g| g.len())
    }

    /// Return true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
