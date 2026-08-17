use crate::parser::behavior::{LanguageBehavior, LanguageId};
use crate::parser::languages::{
    CBehavior, CppBehavior, GoBehavior, JavaBehavior, JavaScriptBehavior, PhpBehavior,
    PythonBehavior, RustBehavior, TypeScriptBehavior,
};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Registry of supported language behaviors.
#[derive(Clone)]
pub struct LanguageRegistry {
    by_extension: HashMap<String, Arc<dyn LanguageBehavior>>,
    by_language_id: HashMap<LanguageId, Arc<dyn LanguageBehavior>>,
    behaviors: Vec<Arc<dyn LanguageBehavior>>,
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageRegistry {
    /// Create a new registry with all 8 supported languages registered.
    pub fn new() -> Self {
        let mut registry = Self {
            by_extension: HashMap::new(),
            by_language_id: HashMap::new(),
            behaviors: Vec::new(),
        };

        registry.register(Arc::new(PythonBehavior));
        registry.register(Arc::new(RustBehavior));
        registry.register(Arc::new(JavaScriptBehavior));
        registry.register(Arc::new(TypeScriptBehavior));
        registry.register(Arc::new(GoBehavior));
        registry.register(Arc::new(CBehavior));
        registry.register(Arc::new(CppBehavior));
        registry.register(Arc::new(JavaBehavior));
        registry.register(Arc::new(PhpBehavior));

        registry
    }

    /// Register a language behavior.
    pub fn register(&mut self, behavior: Arc<dyn LanguageBehavior>) {
        self.by_language_id
            .insert(behavior.language_id(), Arc::clone(&behavior));

        for ext in behavior.extensions() {
            self.by_extension
                .insert((*ext).to_lowercase(), Arc::clone(&behavior));
        }

        self.behaviors.push(behavior);
    }

    /// Detect language behavior by file path and optional file content (for shebang inspection).
    pub fn detect(&self, path: &Path, content: Option<&[u8]>) -> Option<Arc<dyn LanguageBehavior>> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if let Some(behavior) = self.by_extension.get(&ext.to_lowercase()) {
                return Some(Arc::clone(behavior));
            }
        }

        if let Some(bytes) = content {
            if let Some(first_line) = std::str::from_utf8(bytes).ok().and_then(|s| s.lines().next()) {
                if first_line.starts_with("#!") {
                    for behavior in &self.behaviors {
                        if behavior.matches_shebang(first_line) {
                            return Some(Arc::clone(behavior));
                        }
                    }
                }
            }
        }

        None
    }

    /// Lookup behavior directly by language ID.
    pub fn get(&self, id: LanguageId) -> Option<Arc<dyn LanguageBehavior>> {
        self.by_language_id.get(&id).cloned()
    }
}
