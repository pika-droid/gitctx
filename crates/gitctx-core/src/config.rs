use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ConfigHash(pub String);

impl ConfigHash {
    pub fn new(hash: impl Into<String>) -> Self {
        Self(hash.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ConfigHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct IndexConfig {
    pub schema_version: u32,
    pub model_id: String,
    pub tokenizer_name: String,
    pub max_commit_files: usize,
    pub parser_version: u32,
}

impl IndexConfig {
    pub fn new(
        schema_version: u32,
        model_id: impl Into<String>,
        tokenizer_name: impl Into<String>,
        max_commit_files: usize,
        parser_version: u32,
    ) -> Self {
        Self {
            schema_version,
            model_id: model_id.into(),
            tokenizer_name: tokenizer_name.into(),
            max_commit_files,
            parser_version,
        }
    }

    /// Produce a canonical, deterministic JSON representation with sorted keys.
    pub fn canonical_json(&self) -> String {
        let mut map = BTreeMap::new();
        map.insert(
            "max_commit_files",
            serde_json::Value::Number(serde_json::Number::from(self.max_commit_files)),
        );
        map.insert("model_id", serde_json::Value::String(self.model_id.clone()));
        map.insert(
            "parser_version",
            serde_json::Value::Number(serde_json::Number::from(self.parser_version)),
        );
        map.insert(
            "schema_version",
            serde_json::Value::Number(serde_json::Number::from(self.schema_version)),
        );
        map.insert(
            "tokenizer_name",
            serde_json::Value::String(self.tokenizer_name.clone()),
        );

        serde_json::to_string(&map).expect("Failed to serialize canonical config JSON")
    }

    /// Compute deterministic 256-bit BLAKE3 configuration digest over canonical JSON representation.
    pub fn compute_config_hash(&self) -> ConfigHash {
        let canonical = self.canonical_json();
        let mut hasher = blake3::Hasher::new();
        hasher.update(canonical.as_bytes());
        ConfigHash(hasher.finalize().to_hex().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_config_hash() {
        let config1 = IndexConfig {
            schema_version: 1,
            model_id: "all-MiniLM-L6-v2".to_string(),
            tokenizer_name: "bert-base-uncased".to_string(),
            max_commit_files: 500,
            parser_version: 1,
        };

        let config2 = IndexConfig {
            schema_version: 1,
            model_id: "all-MiniLM-L6-v2".to_string(),
            tokenizer_name: "bert-base-uncased".to_string(),
            max_commit_files: 500,
            parser_version: 1,
        };

        assert_eq!(config1.canonical_json(), config2.canonical_json());
        assert_eq!(config1.compute_config_hash(), config2.compute_config_hash());
        assert_eq!(config1.compute_config_hash().as_str().len(), 64);
    }

    #[test]
    fn test_hash_sensitivity_to_modifications() {
        let base_config = IndexConfig {
            schema_version: 1,
            model_id: "all-MiniLM-L6-v2".to_string(),
            tokenizer_name: "bert-base-uncased".to_string(),
            max_commit_files: 500,
            parser_version: 1,
        };
        let base_hash = base_config.compute_config_hash();

        let mut diff_schema = base_config.clone();
        diff_schema.schema_version = 2;
        assert_ne!(base_hash, diff_schema.compute_config_hash());

        let mut diff_model = base_config.clone();
        diff_model.model_id = "bge-small-en-v1.5".to_string();
        assert_ne!(base_hash, diff_model.compute_config_hash());

        let mut diff_tokenizer = base_config.clone();
        diff_tokenizer.tokenizer_name = "bge-tokenizer".to_string();
        assert_ne!(base_hash, diff_tokenizer.compute_config_hash());

        let mut diff_max_files = base_config.clone();
        diff_max_files.max_commit_files = 1000;
        assert_ne!(base_hash, diff_max_files.compute_config_hash());

        let mut diff_parser = base_config.clone();
        diff_parser.parser_version = 2;
        assert_ne!(base_hash, diff_parser.compute_config_hash());
    }
}
