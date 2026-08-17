//! Lexical AST body normalizer per D-01.
//!
//! Normalizes function bodies into a deterministic token sequence:
//! - Strips comments, docstrings, and whitespace.
//! - Canonicalizes string literals (`<str>`) and numeric literals (`<num>`).
//! - Preserves language control keywords (`<kw:...>`) and structural operators (`<op:...>`).
//! - Maps local variable identifiers to positional placeholders (`var_0`, `var_1`, ...).

use crate::template::is_keyword;
use std::collections::HashMap;

/// Lexical AST body token normalizer.
#[derive(Debug, Default, Clone)]
pub struct AstNormalizer;

impl AstNormalizer {
    /// Create a new AST normalizer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Normalize raw source code text into canonical tokens.
    pub fn normalize(&self, source: &str) -> Vec<String> {
        let chars: Vec<char> = source.chars().collect();
        let len = chars.len();
        let mut i = 0;

        let mut tokens = Vec::new();
        let mut var_map: HashMap<String, String> = HashMap::new();

        while i < len {
            let c = chars[i];

            // 1. Skip whitespace
            if c.is_whitespace() {
                i += 1;
                continue;
            }

            // 2. Comments
            // Python/Shell line comment `#`
            if c == '#' {
                i += 1;
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }

            // Line comments `//` or Block comments `/* */`
            if c == '/' && i + 1 < len {
                if chars[i + 1] == '/' {
                    i += 2;
                    while i < len && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                } else if chars[i + 1] == '*' {
                    i += 2;
                    while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                        i += 1;
                    }
                    if i + 1 < len {
                        i += 2;
                    } else {
                        i = len;
                    }
                    continue;
                }
            }

            // HTML/XML comments `<!-- -->`
            if c == '<' && i + 3 < len && chars[i + 1] == '!' && chars[i + 2] == '-' && chars[i + 3] == '-' {
                i += 4;
                while i + 2 < len && !(chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>') {
                    i += 1;
                }
                if i + 2 < len {
                    i += 3;
                } else {
                    i = len;
                }
                continue;
            }

            // 3. String literals & Docstrings
            // Triple-quoted Python strings `"""` or `'''`
            if (c == '"' || c == '\'') && i + 2 < len && chars[i + 1] == c && chars[i + 2] == c {
                let quote_char = c;
                i += 3;
                while i + 2 < len && !(chars[i] == quote_char && chars[i + 1] == quote_char && chars[i + 2] == quote_char) {
                    if chars[i] == '\\' {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i + 2 < len {
                    i += 3;
                } else {
                    i = len;
                }
                tokens.push("<str>".to_string());
                continue;
            }

            // Single/Double/Backtick quoted strings
            if c == '"' || c == '\'' || c == '`' {
                let quote_char = c;
                i += 1;
                while i < len && chars[i] != quote_char {
                    if chars[i] == '\\' && i + 1 < len {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < len {
                    i += 1; // consume closing quote
                }
                tokens.push("<str>".to_string());
                continue;
            }

            // 4. Numeric literals
            if c.is_ascii_digit() || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
                let start = i;
                if c == '0' && i + 1 < len && (chars[i + 1] == 'x' || chars[i + 1] == 'X' || chars[i + 1] == 'b' || chars[i + 1] == 'B' || chars[i + 1] == 'o' || chars[i + 1] == 'O') {
                    i += 2;
                    while i < len && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                        i += 1;
                    }
                } else {
                    while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                        // Handle scientific notation e+ / e-
                        if (chars[i] == 'e' || chars[i] == 'E') && i + 1 < len && (chars[i + 1] == '+' || chars[i + 1] == '-') {
                            i += 2;
                            continue;
                        }
                        i += 1;
                    }
                }
                let _num_str: String = chars[start..i].iter().collect();
                tokens.push("<num>".to_string());
                continue;
            }

            // 5. Identifiers, Keywords, and Variables
            if c.is_ascii_alphabetic() || c == '_' || c == '$' {
                let start = i;
                while i < len && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '$') {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                if is_keyword(&ident) {
                    tokens.push(format!("<kw:{}>", ident.to_ascii_lowercase()));
                } else {
                    let next_idx = var_map.len();
                    let placeholder = var_map.entry(ident).or_insert_with(|| format!("var_{next_idx}"));
                    tokens.push(placeholder.clone());
                }
                continue;
            }

            // 6. Multi-character operators
            // 3-character operators
            if i + 2 < len {
                let op3: String = chars[i..i + 3].iter().collect();
                if matches!(op3.as_str(), "..." | "<<=" | ">>=" | "===" | "!==" | "..=") {
                    tokens.push(format!("<op:{op3}>"));
                    i += 3;
                    continue;
                }
            }

            // 2-character operators
            if i + 1 < len {
                let op2: String = chars[i..i + 2].iter().collect();
                if matches!(
                    op2.as_str(),
                    "::" | "->"
                        | "=>"
                        | "=="
                        | "!="
                        | "<="
                        | ">="
                        | "+="
                        | "-="
                        | "*="
                        | "/="
                        | "%="
                        | "&="
                        | "|="
                        | "^="
                        | "&&"
                        | "||"
                        | "<<"
                        | ">>"
                        | ".."
                        | "??"
                        | "?."
                        | "++"
                        | "--"
                ) {
                    tokens.push(format!("<op:{op2}>"));
                    i += 2;
                    continue;
                }
            }

            // Single-character operators & delimiters
            let op1 = c.to_string();
            tokens.push(format!("<op:{op1}>"));
            i += 1;
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalizer_m5_formatting_invariance() {
        let normalizer = AstNormalizer::new();
        let code1 = "def add(a: int, b: int) -> int:\n    return a + b\n";
        let code2 = "def   add  ( a : int , b : int ) -> int : \n\n    # Comment\n    return   a   +   b \n";

        let tokens1 = normalizer.normalize(code1);
        let tokens2 = normalizer.normalize(code2);

        assert_eq!(tokens1, tokens2);
    }

    #[test]
    fn test_normalizer_m6_variable_renaming_invariance() {
        let normalizer = AstNormalizer::new();
        let code1 = r#"
        def process(data, factor):
            temp = data * factor
            result = temp + 10
            return result
        "#;
        let code2 = r#"
        def process(input_val, multiplier):
            intermediate = input_val * multiplier
            output = intermediate + 10
            return output
        "#;

        let tokens1 = normalizer.normalize(code1);
        let tokens2 = normalizer.normalize(code2);

        assert_eq!(tokens1, tokens2);
    }
}
