use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct QualName(pub String);

impl QualName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn segments(&self) -> Vec<&str> {
        self.0.split("::").collect()
    }

    pub fn parent(&self) -> Option<String> {
        let segs = self.segments();
        if segs.len() > 1 {
            Some(segs[..segs.len() - 1].join("::"))
        } else {
            None
        }
    }

    pub fn leaf(&self) -> &str {
        self.0.rsplit("::").next().unwrap_or(&self.0)
    }

    pub fn is_method(&self) -> bool {
        self.segments().len() >= 3
    }
}

impl std::fmt::Display for QualName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for QualName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for QualName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qualname_parsing() {
        let func = QualName::new("module::submodule::function_name");
        assert_eq!(func.leaf(), "function_name");
        assert_eq!(func.parent(), Some("module::submodule".to_string()));
        assert!(func.is_method());

        let top_level = QualName::new("main");
        assert_eq!(top_level.leaf(), "main");
        assert_eq!(top_level.parent(), None);
        assert!(!top_level.is_method());

        let simple_func = QualName::new("module::function_name");
        assert_eq!(simple_func.leaf(), "function_name");
        assert_eq!(simple_func.parent(), Some("module".to_string()));
        assert!(!simple_func.is_method());
    }
}
