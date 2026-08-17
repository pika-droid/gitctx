use crate::error::Result;
use crate::parser::behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
use tree_sitter::{Node, Tree};

pub struct JavaScriptBehavior;

impl LanguageBehavior for JavaScriptBehavior {
    fn language_id(&self) -> LanguageId {
        LanguageId::JavaScript
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["js", "jsx", "mjs", "cjs"]
    }

    fn matches_shebang(&self, first_line: &str) -> bool {
        first_line.starts_with("#!") && first_line.contains("node")
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::from(tree_sitter_javascript::LANGUAGE)
    }

    fn extract_symbols(&self, code: &[u8], tree: &Tree) -> Result<Vec<RawFunctionSymbol>> {
        let mut symbols = Vec::new();
        let mut cursor = tree.walk();
        let mut scope_stack: Vec<String> = Vec::new();

        Self::walk_node(tree.root_node(), code, &mut scope_stack, &mut symbols, &mut cursor);
        Ok(symbols)
    }
}

impl JavaScriptBehavior {
    fn walk_node(
        node: Node,
        code: &[u8],
        scope_stack: &mut Vec<String>,
        symbols: &mut Vec<RawFunctionSymbol>,
        cursor: &mut tree_sitter::TreeCursor,
    ) {
        let kind = node.kind();
        let is_class = kind == "class_declaration" || kind == "class";
        let is_func = kind == "function_declaration" || kind == "method_definition" || kind == "generator_function_declaration";

        if is_class {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name_str) = name_node.utf8_text(code) {
                    scope_stack.push(name_str.to_string());
                }
            }
        } else if is_func {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name_str) = name_node.utf8_text(code) {
                    let is_method = !scope_stack.is_empty();
                    let qualname = if scope_stack.is_empty() {
                        name_str.to_string()
                    } else {
                        format!("{}.{}", scope_stack.join("."), name_str)
                    };

                    let signature = Self::extract_js_signature(node, code, name_str);
                    let docstring = Self::extract_js_docstring(node, code);

                    let start_byte = node.start_byte();
                    let end_byte = node.end_byte();
                    let start_line = u32::try_from(node.start_position().row + 1).unwrap_or(1);
                    let end_line = u32::try_from(node.end_position().row + 1).unwrap_or(1);

                    symbols.push(RawFunctionSymbol {
                        name: name_str.to_string(),
                        qualname,
                        signature,
                        docstring,
                        start_byte,
                        end_byte,
                        start_line,
                        end_line,
                        is_method,
                    });
                }
            }
        }

        if cursor.goto_first_child() {
            loop {
                Self::walk_node(cursor.node(), code, scope_stack, symbols, cursor);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }

        if is_class {
            scope_stack.pop();
        }
    }

    fn extract_js_signature(node: Node, code: &[u8], name: &str) -> String {
        let params = node
            .child_by_field_name("parameters")
            .and_then(|p| p.utf8_text(code).ok())
            .unwrap_or("()");

        let is_async = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "async");
        let async_prefix = if is_async { "async " } else { "" };

        format!("{async_prefix}function {name}{params}")
    }

    fn extract_js_docstring(node: Node, code: &[u8]) -> Option<String> {
        let mut prev = node.prev_sibling();
        while let Some(sibling) = prev {
            if sibling.kind() == "comment" {
                if let Ok(comment_text) = sibling.utf8_text(code) {
                    if comment_text.starts_with("/**") {
                        let clean = comment_text
                            .trim_start_matches("/**")
                            .trim_end_matches("*/")
                            .lines()
                            .map(|l| l.trim().trim_start_matches('*').trim())
                            .filter(|l| !l.is_empty())
                            .collect::<Vec<_>>()
                            .join("\n");
                        return Some(clean);
                    }
                }
                prev = sibling.prev_sibling();
            } else {
                break;
            }
        }
        None
    }
}
