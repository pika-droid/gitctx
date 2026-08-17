//! Keyword exclusion sets for 8 supported languages and common boilerplate tokens.
//!
//! Used by `IdentifierExtractor` to filter out language-reserved keywords and stop
//! words during symmetric diff token extraction per D-11.

use std::collections::HashSet;
use std::sync::LazyLock;

static KEYWORDS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();

    // Python keywords & builtins
    s.extend([
        "def", "class", "return", "import", "from", "if", "else", "elif", "for", "while", "try",
        "except", "finally", "with", "as", "pass", "break", "continue", "raise", "yield", "lambda",
        "and", "or", "not", "in", "is", "none", "true", "false", "self", "cls", "async", "await",
        "nonlocal", "global", "assert", "del",
    ]);

    // Rust keywords & primitives
    s.extend([
        "fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl", "trait", "where",
        "match", "const", "static", "ref", "move", "type", "crate", "super", "extern", "unsafe",
        "dyn", "box", "loop",
    ]);

    // JavaScript / TypeScript keywords
    s.extend([
        "function",
        "var",
        "export",
        "default",
        "interface",
        "extends",
        "implements",
        "switch",
        "case",
        "of",
        "catch",
        "throw",
        "new",
        "this",
        "typeof",
        "instanceof",
        "void",
        "null",
        "undefined",
        "declare",
        "abstract",
        "readonly",
        "private",
        "protected",
        "public",
        "override",
    ]);

    // Go keywords & builtins
    s.extend([
        "package",
        "func",
        "range",
        "goto",
        "fallthrough",
        "go",
        "defer",
        "select",
        "chan",
        "map",
        "nil",
        "make",
        "len",
        "cap",
        "append",
        "copy",
        "delete",
        "panic",
        "recover",
    ]);

    // C / C++ keywords
    s.extend([
        "auto",
        "char",
        "double",
        "float",
        "int",
        "long",
        "register",
        "short",
        "signed",
        "sizeof",
        "typedef",
        "union",
        "unsigned",
        "volatile",
        "virtual",
        "friend",
        "inline",
        "template",
        "typename",
        "namespace",
        "using",
        "nullptr",
        "constexpr",
        "decltype",
        "noexcept",
        "static_assert",
        "explicit",
    ]);

    // Java keywords
    s.extend([
        "boolean",
        "byte",
        "native",
        "strictfp",
        "synchronized",
        "throws",
        "transient",
        "record",
        "sealed",
        "permits",
    ]);

    // PHP keywords
    s.extend([
        "callable",
        "die",
        "echo",
        "elseif",
        "empty",
        "enddeclare",
        "endfor",
        "endforeach",
        "endif",
        "endswitch",
        "endwhile",
        "eval",
        "exit",
        "include",
        "include_once",
        "insteadof",
        "isset",
        "parent",
        "print",
        "require",
        "require_once",
        "unset",
        "xor",
    ]);

    // Common English boilerplate & generic types
    s.extend([
        "string", "bool", "list", "dict", "tuple", "set", "the", "that", "these", "those", "are",
        "was", "were", "has", "have", "had", "will", "would", "could", "should", "can",
    ]);

    s
});

/// Returns true if the token is a keyword or common boilerplate stop word.
#[must_use]
pub fn is_keyword(token: &str) -> bool {
    let lower = token.to_ascii_lowercase();
    KEYWORDS.contains(lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_keyword() {
        assert!(is_keyword("def"));
        assert!(is_keyword("Def"));
        assert!(is_keyword("fn"));
        assert!(!is_keyword("goroutine"));
        assert!(!is_keyword("my_function"));
        assert!(!is_keyword("calculate_total"));
    }
}
