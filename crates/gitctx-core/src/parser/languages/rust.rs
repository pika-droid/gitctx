use crate::error::Result;
use crate::parser::behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
use tree_sitter::{Node, Tree};

pub struct RustBehavior;

impl LanguageBehavior for RustBehavior {
    fn language_id(&self) -> LanguageId {
        LanguageId::Rust
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn matches_shebang(&self, _first_line: &str) -> bool {
        false
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::from(tree_sitter_rust::LANGUAGE)
    }

    fn extract_symbols(&self, code: &[u8], tree: &Tree) -> Result<Vec<RawFunctionSymbol>> {
        let mut symbols = Vec::new();
        let mut cursor = tree.walk();
        let mut scope_stack: Vec<String> = Vec::new();

        Self::walk_node(tree.root_node(), code, &mut scope_stack, &mut symbols, &mut cursor);
        Ok(symbols)
    }
}

impl RustBehavior {
    fn walk_node(
        node: Node,
        code: &[u8],
        scope_stack: &mut Vec<String>,
        symbols: &mut Vec<RawFunctionSymbol>,
        cursor: &mut tree_sitter::TreeCursor,
    ) {
        let kind = node.kind();
        let is_impl = kind == "impl_item";
        let is_trait = kind == "trait_item";
        let is_func = kind == "function_item";

        if is_impl {
            if let Some(type_node) = node.child_by_field_name("type") {
                if let Ok(type_str) = type_node.utf8_text(code) {
                    scope_stack.push(type_str.to_string());
                }
            }
        } else if is_trait {
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
                        format!("{}::{}", scope_stack.join("::"), name_str)
                    };

                    let signature = Self::extract_rust_signature(node, code);
                    let docstring = Self::extract_rust_docstring(node, code);

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

        if is_impl || is_trait {
            scope_stack.pop();
        }
    }

    fn extract_rust_signature(node: Node, code: &[u8]) -> String {
        let params = node
            .child_by_field_name("parameters")
            .and_then(|p| p.utf8_text(code).ok())
            .unwrap_or("()");

        let return_type = node
            .child_by_field_name("return_type")
            .and_then(|r| r.utf8_text(code).ok())
            .map(|r| {
                if r.starts_with("->") {
                    format!(" {r}")
                } else {
                    format!(" -> {r}")
                }
            })
            .unwrap_or_default();

        let name_str = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(code).ok())
            .unwrap_or("fn");

        let is_pub = node
            .children(&mut node.walk())
            .any(|c| c.kind() == "visibility_modifier");
        let pub_prefix = if is_pub { "pub " } else { "" };

        format!("{pub_prefix}fn {name_str}{params}{return_type}")
    }

    fn extract_rust_docstring(node: Node, code: &[u8]) -> Option<String> {
        let mut docs = Vec::new();
        let mut prev = node.prev_sibling();

        while let Some(sibling) = prev {
            if sibling.kind() == "line_comment" {
                if let Ok(comment_text) = sibling.utf8_text(code) {
                    if comment_text.starts_with("///") {
                        let clean = comment_text.trim_start_matches("///").trim();
                        docs.push(clean.to_string());
                    }
                }
                prev = sibling.prev_sibling();
            } else {
                break;
            }
        }

        if docs.is_empty() {
            None
        } else {
            docs.reverse();
            Some(docs.join("\n"))
        }
    }
}
