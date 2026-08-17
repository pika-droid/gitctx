use crate::error::Result;
use crate::parser::behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
use tree_sitter::{Node, Tree};

pub struct CBehavior;

impl LanguageBehavior for CBehavior {
    fn language_id(&self) -> LanguageId {
        LanguageId::C
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["c", "h"]
    }

    fn matches_shebang(&self, _first_line: &str) -> bool {
        false
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::from(tree_sitter_c::LANGUAGE)
    }

    fn extract_symbols(&self, code: &[u8], tree: &Tree) -> Result<Vec<RawFunctionSymbol>> {
        let mut symbols = Vec::new();
        let mut cursor = tree.walk();

        Self::walk_node(tree.root_node(), code, &mut symbols, &mut cursor);
        Ok(symbols)
    }
}

impl CBehavior {
    fn walk_node(
        node: Node,
        code: &[u8],
        symbols: &mut Vec<RawFunctionSymbol>,
        cursor: &mut tree_sitter::TreeCursor,
    ) {
        if node.kind() == "function_definition" {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name_str) = Self::find_function_name(declarator, code) {
                    let signature = Self::extract_c_signature(node, code);
                    let docstring = Self::extract_c_docstring(node, code);

                    let start_byte = node.start_byte();
                    let end_byte = node.end_byte();
                    let start_line = u32::try_from(node.start_position().row + 1).unwrap_or(1);
                    let end_line = u32::try_from(node.end_position().row + 1).unwrap_or(1);

                    symbols.push(RawFunctionSymbol {
                        name: name_str.clone(),
                        qualname: name_str,
                        signature,
                        docstring,
                        start_byte,
                        end_byte,
                        start_line,
                        end_line,
                        is_method: false,
                    });
                }
            }
        }

        if cursor.goto_first_child() {
            loop {
                Self::walk_node(cursor.node(), code, symbols, cursor);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
            cursor.goto_parent();
        }
    }

    fn find_function_name(node: Node, code: &[u8]) -> Option<String> {
        if node.kind() == "identifier" {
            return node.utf8_text(code).ok().map(ToString::to_string);
        }
        if let Some(direct_declarator) = node.child_by_field_name("declarator") {
            return Self::find_function_name(direct_declarator, code);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(name) = Self::find_function_name(child, code) {
                return Some(name);
            }
        }
        None
    }

    fn extract_c_signature(node: Node, code: &[u8]) -> String {
        let return_type = node
            .child_by_field_name("type")
            .and_then(|t| t.utf8_text(code).ok())
            .unwrap_or("void");

        let declarator = node
            .child_by_field_name("declarator")
            .and_then(|d| d.utf8_text(code).ok())
            .unwrap_or("func()");

        format!("{return_type} {declarator}")
    }

    fn extract_c_docstring(node: Node, code: &[u8]) -> Option<String> {
        let mut prev = node.prev_sibling();
        while let Some(sibling) = prev {
            if sibling.kind() == "comment" {
                if let Ok(comment_text) = sibling.utf8_text(code) {
                    let clean = comment_text
                        .trim_start_matches("/*")
                        .trim_end_matches("*/")
                        .trim_start_matches("//")
                        .trim()
                        .to_string();
                    return Some(clean);
                }
                prev = sibling.prev_sibling();
            } else {
                break;
            }
        }
        None
    }
}
