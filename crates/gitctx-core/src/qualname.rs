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

    /// Split qualified name by standard double-colon `::` delimiter.
    pub fn segments(&self) -> Vec<&str> {
        self.0.split("::").collect()
    }

    /// Parent namespace, class, or enclosing function qualname.
    pub fn parent(&self) -> Option<String> {
        let segs = self.segments();
        if segs.len() > 1 {
            Some(segs[..segs.len() - 1].join("::"))
        } else {
            None
        }
    }

    /// Leaf symbol identifier name.
    pub fn leaf(&self) -> &str {
        self.0.rsplit("::").next().unwrap_or(&self.0)
    }

    /// True if the symbol is enclosed in a class or struct scope.
    pub fn is_method(&self) -> bool {
        self.segments().len() >= 2
            && !self.leaf().starts_with('<')
            && self.parent().map_or(false, |p| !p.ends_with('>'))
    }

    /// True if the symbol represents a nested function or anonymous closure.
    pub fn is_nested_or_closure(&self) -> bool {
        self.0.contains("::<")
    }

    /// True if the symbol is an anonymous closure or lambda marker.
    pub fn is_closure(&self) -> bool {
        self.0.contains("::<closure@")
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
        let method = QualName::new("Calculator::add");
        assert_eq!(method.leaf(), "add");
        assert_eq!(method.parent(), Some("Calculator".to_string()));
        assert!(method.is_method());
        assert!(!method.is_nested_or_closure());

        let top_level = QualName::new("main");
        assert_eq!(top_level.leaf(), "main");
        assert_eq!(top_level.parent(), None);
        assert!(!top_level.is_method());
        assert!(!top_level.is_nested_or_closure());

        let nested = QualName::new("outer_func::<nested_func>");
        assert_eq!(nested.leaf(), "<nested_func>");
        assert_eq!(nested.parent(), Some("outer_func".to_string()));
        assert!(!nested.is_method());
        assert!(nested.is_nested_or_closure());
        assert!(!nested.is_closure());

        let closure = QualName::new("handler::<closure@L42>");
        assert_eq!(closure.leaf(), "<closure@L42>");
        assert_eq!(closure.parent(), Some("handler".to_string()));
        assert!(!closure.is_method());
        assert!(closure.is_nested_or_closure());
        assert!(closure.is_closure());
    }
}
