use crate::error::Result;
use crate::parser::behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
use tree_sitter::{Node, Tree};

pub struct CppBehavior;

impl LanguageBehavior for CppBehavior {
    fn language_id(&self) -> LanguageId {
        LanguageId::Cpp
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["cpp", "cxx", "cc", "hpp", "hxx", "hh"]
    }

    fn matches_shebang(&self, _first_line: &str) -> bool {
        false
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::from(tree_sitter_cpp::LANGUAGE)
    }

    fn extract_symbols(&self, code: &[u8], tree: &Tree) -> Result<Vec<RawFunctionSymbol>> {
        let mut symbols = Vec::new();
        let mut cursor = tree.walk();
        let mut scope_stack: Vec<String> = Vec::new();

        Self::walk_node(tree.root_node(), code, &mut scope_stack, &mut symbols, &mut cursor);
        Ok(symbols)
    }
}

impl CppBehavior {
    fn walk_node(
        node: Node,
        code: &[u8],
        scope_stack: &mut Vec<String>,
        symbols: &mut Vec<RawFunctionSymbol>,
        cursor: &mut tree_sitter::TreeCursor,
    ) {
        let kind = node.kind();
        let is_class = kind == "class_specifier" || kind == "struct_specifier" || kind == "namespace_definition";
        let is_func = kind == "function_definition";

        if is_class {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name_str) = name_node.utf8_text(code) {
                    scope_stack.push(name_str.to_string());
                }
            }
        } else if is_func {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                if let Some(name_str) = Self::find_function_name(declarator, code) {
                    let is_method = !scope_stack.is_empty();
                    let qualname = if scope_stack.is_empty() {
                        name_str.clone()
                    } else {
                        format!("{}::{}", scope_stack.join("::"), name_str)
                    };

                    let signature = Self::extract_cpp_signature(node, code);
                    let docstring = Self::extract_cpp_docstring(node, code);

                    let start_byte = node.start_byte();
                    let end_byte = node.end_byte();
                    let start_line = u32::try_from(node.start_position().row + 1).unwrap_or(1);
                    let end_line = u32::try_from(node.end_position().row + 1).unwrap_or(1);

                    symbols.push(RawFunctionSymbol {
                        name: name_str,
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

    fn find_function_name(node: Node, code: &[u8]) -> Option<String> {
        if node.kind() == "identifier" || node.kind() == "field_identifier" {
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

    fn extract_cpp_signature(node: Node, code: &[u8]) -> String {
        let return_type = node
            .child_by_field_name("type")
            .and_then(|t| t.utf8_text(code).ok())
            .unwrap_or("auto");

        let declarator = node
            .child_by_field_name("declarator")
            .and_then(|d| d.utf8_text(code).ok())
            .unwrap_or("func()");

        format!("{return_type} {declarator}")
    }

    fn extract_cpp_docstring(node: Node, code: &[u8]) -> Option<String> {
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
