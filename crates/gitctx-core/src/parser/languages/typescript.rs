use crate::error::Result;
use crate::parser::behavior::{LanguageBehavior, LanguageId, RawFunctionSymbol};
use tree_sitter::{Node, Tree};

pub struct TypeScriptBehavior;

impl LanguageBehavior for TypeScriptBehavior {
    fn language_id(&self) -> LanguageId {
        LanguageId::TypeScript
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "tsx", "mts", "cts"]
    }

    fn matches_shebang(&self, first_line: &str) -> bool {
        first_line.starts_with("#!")
            && (first_line.contains("deno")
                || first_line.contains("bun")
                || first_line.contains("ts-node"))
    }

    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter::Language::from(tree_sitter_typescript::LANGUAGE_TYPESCRIPT)
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

impl TypeScriptBehavior {
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
        let is_class = kind == "class_declaration"
            || kind == "interface_declaration"
            || kind == "enum_declaration";
        let is_func = kind == "function_declaration"
            || kind == "method_definition"
            || kind == "function_signature";
        let is_arrow = kind == "arrow_function";

        let mut pushed_scope = false;

        if is_class {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name_str) = name_node.utf8_text(code) {
                    scope_stack.push(ScopeEntry {
                        name: name_str.to_string(),
                        is_class: true,
                    });
                    pushed_scope = true;
                }
            }
        } else if is_func {
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Ok(name_str) = name_node.utf8_text(code) {
                    let is_method = scope_stack.last().is_some_and(|e| e.is_class);
                    let qualname = Self::build_qualname(scope_stack, name_str, false);
                    let signature = Self::extract_ts_signature(node, code, name_str);
                    let docstring = Self::extract_ts_docstring(node, code);

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

                    scope_stack.push(ScopeEntry {
                        name: name_str.to_string(),
                        is_class: false,
                    });
                    pushed_scope = true;
                }
            }
        } else if is_arrow {
            let start_line = u32::try_from(node.start_position().row + 1).unwrap_or(1);
            let end_line = u32::try_from(node.end_position().row + 1).unwrap_or(1);
            let closure_marker = format!("closure@L{start_line}");
            let qualname = Self::build_qualname(scope_stack, &closure_marker, true);
            let signature = node
                .utf8_text(code)
                .map_or_else(|_| "() => { ... }".to_string(), ToString::to_string);

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
        }
    }

    fn extract_ts_signature(node: Node, code: &[u8], name: &str) -> String {
        let params = node
            .child_by_field_name("parameters")
            .and_then(|p| p.utf8_text(code).ok())
            .unwrap_or("()");

        let return_type = node
            .child_by_field_name("return_type")
            .and_then(|r| r.utf8_text(code).ok())
            .map(|r| format!(": {r}"))
            .unwrap_or_default();

        let is_async = node.children(&mut node.walk()).any(|c| c.kind() == "async");
        let async_prefix = if is_async { "async " } else { "" };

        format!("{async_prefix}function {name}{params}{return_type}")
    }

    fn extract_ts_docstring(node: Node, code: &[u8]) -> Option<String> {
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
