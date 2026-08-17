use crate::error::Result;
use crate::parser::behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
use tree_sitter::{Node, Tree};

pub struct GoBehavior;

impl LanguageBehavior for GoBehavior {
    fn language_id(&self) -> LanguageId {
        LanguageId::Go
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn matches_shebang(&self, _first_line: &str) -> bool {
        false
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::from(tree_sitter_go::LANGUAGE)
    }

    fn extract_symbols(&self, code: &[u8], tree: &Tree) -> Result<Vec<RawFunctionSymbol>> {
        let mut symbols = Vec::new();
        let mut cursor = tree.walk();
        let mut scope_stack: Vec<ScopeEntry> = Vec::new();

        Self::walk_node(
            tree.root_node(),
            code,
            &mut scope_stack,
            &mut symbols,
            &mut cursor,
        );
        Ok(symbols)
    }
}

#[derive(Clone, Debug)]
struct ScopeEntry {
    name: String,
    is_class: bool,
}

impl GoBehavior {
    fn build_qualname(scope_stack: &[ScopeEntry], leaf_name: &str, is_closure: bool) -> String {
        if scope_stack.is_empty() {
            if is_closure {
                format!("<{leaf_name}>")
            } else {
                leaf_name.to_string()
            }
        } else {
            let mut path = String::new();
            for (i, entry) in scope_stack.iter().enumerate() {
                if i == 0 {
                    path.push_str(&entry.name);
                } else {
                    let prev_is_class = scope_stack[i - 1].is_class;
                    if prev_is_class {
                        path.push_str("::");
                        path.push_str(&entry.name);
                    } else {
                        path.push_str("::<");
                        path.push_str(&entry.name);
                        path.push('>');
                    }
                }
            }
            let last_is_class = scope_stack.last().is_some_and(|e| e.is_class);
            if last_is_class && !is_closure {
                format!("{path}::{leaf_name}")
            } else {
                format!("{path}::<{leaf_name}>")
            }
        }
    }

    fn walk_node(
        node: Node,
        code: &[u8],
        scope_stack: &mut Vec<ScopeEntry>,
        symbols: &mut Vec<RawFunctionSymbol>,
        cursor: &mut tree_sitter::TreeCursor,
    ) {
        let kind = node.kind();
        let is_func = kind == "function_declaration";
        let is_method = kind == "method_declaration";
        let is_func_lit = kind == "func_literal";

        let mut pushed_scope = false;

        if is_func || is_method {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name_str) = name_node.utf8_text(code) {
                    let receiver_type = if is_method {
                        node.child_by_field_name("receiver")
                            .and_then(|r| Self::extract_go_receiver_type(r, code))
                    } else {
                        None
                    };

                    let (qualname, method_flag) = if let Some(recv) = &receiver_type {
                        let mut temp_scope = scope_stack.clone();
                        temp_scope.push(ScopeEntry {
                            name: recv.clone(),
                            is_class: true,
                        });
                        (Self::build_qualname(&temp_scope, name_str, false), true)
                    } else {
                        let method_flag = scope_stack.last().is_some_and(|e| e.is_class);
                        (
                            Self::build_qualname(scope_stack, name_str, false),
                            method_flag,
                        )
                    };

                    let signature = Self::extract_go_signature(node, code, name_str, is_method);
                    let docstring = Self::extract_go_docstring(node, code);

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
                        is_method: method_flag,
                    });

                    if let Some(recv) = &receiver_type {
                        scope_stack.push(ScopeEntry {
                            name: recv.clone(),
                            is_class: true,
                        });
                    }
                    scope_stack.push(ScopeEntry {
                        name: name_str.to_string(),
                        is_class: false,
                    });
                    pushed_scope = true;
                }
            }
        } else if is_func_lit {
            let start_line = u32::try_from(node.start_position().row + 1).unwrap_or(1);
            let end_line = u32::try_from(node.end_position().row + 1).unwrap_or(1);
            let closure_marker = format!("closure@L{start_line}");
            let qualname = Self::build_qualname(scope_stack, &closure_marker, true);
            let signature = "func() { ... }".to_string();

            symbols.push(RawFunctionSymbol {
                name: format!("<{closure_marker}>"),
                qualname,
                signature,
                docstring: None,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_line,
                end_line,
                is_method: false,
            });
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

        if pushed_scope {
            scope_stack.pop();
            // If we pushed receiver as well, pop it
            if is_method {
                scope_stack.pop();
            }
        }
    }

    fn extract_go_receiver_type(receiver_node: Node, code: &[u8]) -> Option<String> {
        let mut cursor = receiver_node.walk();
        for child in receiver_node.children(&mut cursor) {
            if child.kind() == "parameter_declaration" {
                if let Some(type_node) = child.child_by_field_name("type") {
                    if let Ok(type_str) = type_node.utf8_text(code) {
                        return Some(type_str.trim_start_matches('*').to_string());
                    }
                }
            }
        }
        None
    }

    fn extract_go_signature(node: Node, code: &[u8], name: &str, is_method: bool) -> String {
        let recv_text = if is_method {
            node.child_by_field_name("receiver")
                .and_then(|r| r.utf8_text(code).ok())
                .map(|r| format!("{r} "))
                .unwrap_or_default()
        } else {
            String::new()
        };

        let params = node
            .child_by_field_name("parameters")
            .and_then(|p| p.utf8_text(code).ok())
            .unwrap_or("()");

        let result = node
            .child_by_field_name("result")
            .and_then(|r| r.utf8_text(code).ok())
            .map(|r| format!(" {r}"))
            .unwrap_or_default();

        format!("func {recv_text}{name}{params}{result}")
    }

    fn extract_go_docstring(node: Node, code: &[u8]) -> Option<String> {
        let mut docs = Vec::new();
        let mut prev = node.prev_sibling();

        while let Some(sibling) = prev {
            if sibling.kind() == "comment" {
                if let Ok(comment_text) = sibling.utf8_text(code) {
                    if comment_text.starts_with("//") {
                        let clean = comment_text.trim_start_matches("//").trim();
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
