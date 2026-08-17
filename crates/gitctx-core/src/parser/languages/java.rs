use crate::error::Result;
use crate::parser::behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
use tree_sitter::{Node, Tree};

pub struct JavaBehavior;

impl LanguageBehavior for JavaBehavior {
    fn language_id(&self) -> LanguageId {
        LanguageId::Java
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn matches_shebang(&self, _first_line: &str) -> bool {
        false
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::from(tree_sitter_java::LANGUAGE)
    }

    fn extract_symbols(&self, code: &[u8], tree: &Tree) -> Result<Vec<RawFunctionSymbol>> {
        let mut symbols = Vec::new();
        let mut cursor = tree.walk();
        let mut scope_stack: Vec<String> = Vec::new();

        Self::walk_node(tree.root_node(), code, &mut scope_stack, &mut symbols, &mut cursor);
        Ok(symbols)
    }
}

impl JavaBehavior {
    fn walk_node(
        node: Node,
        code: &[u8],
        scope_stack: &mut Vec<String>,
        symbols: &mut Vec<RawFunctionSymbol>,
        cursor: &mut tree_sitter::TreeCursor,
    ) {
        let kind = node.kind();
        let is_class = kind == "class_declaration" || kind == "interface_declaration" || kind == "enum_declaration" || kind == "record_declaration";
        let is_func = kind == "method_declaration" || kind == "constructor_declaration";

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

                    let signature = Self::extract_java_signature(node, code, name_str);
                    let docstring = Self::extract_java_docstring(node, code);

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

    fn extract_java_signature(node: Node, code: &[u8], name: &str) -> String {
        let return_type = node
            .child_by_field_name("type")
            .and_then(|t| t.utf8_text(code).ok())
            .map(|t| format!("{t} "))
            .unwrap_or_default();

        let params = node
            .child_by_field_name("parameters")
            .and_then(|p| p.utf8_text(code).ok())
            .unwrap_or("()");

        let modifiers = node
            .children(&mut node.walk())
            .filter(|c| c.kind() == "modifiers")
            .filter_map(|m| m.utf8_text(code).ok())
            .map(|m| format!("{m} "))
            .next()
            .unwrap_or_default();

        format!("{modifiers}{return_type}{name}{params}")
    }

    fn extract_java_docstring(node: Node, code: &[u8]) -> Option<String> {
        let mut prev = node.prev_sibling();
        while let Some(sibling) = prev {
            if sibling.kind() == "block_comment" {
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
