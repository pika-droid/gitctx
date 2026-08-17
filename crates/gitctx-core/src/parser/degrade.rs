use crate::model::ParseStatus;
use crate::parser::behavior::RawFunctionSymbol;
use tree_sitter::{Node, Tree};

/// Extracted parsing result with status classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedFileSymbols {
    pub symbols: Vec<RawFunctionSymbol>,
    pub status: ParseStatus,
    pub error_byte_ranges: Vec<(usize, usize)>,
}

/// Helper for analyzing tree-sitter error nodes and degradation status.
pub struct DegradationHandler;

impl DegradationHandler {
    /// Inspect tree-sitter AST for errors and collect error ranges.
    pub fn analyze(tree: &Tree) -> (bool, Vec<(usize, usize)>) {
        let root = tree.root_node();
        if !root.has_error() {
            return (false, Vec::new());
        }

        let mut error_ranges = Vec::new();
        Self::collect_error_ranges(root, &mut error_ranges);
        (true, error_ranges)
    }

    fn collect_error_ranges(node: Node, error_ranges: &mut Vec<(usize, usize)>) {
        if node.is_error() || node.is_missing() {
            error_ranges.push((node.start_byte(), node.end_byte()));
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.has_error() {
                Self::collect_error_ranges(child, error_ranges);
            }
        }
    }

    /// Check if a modified byte range intersects any syntax error ranges.
    pub fn intersects_error(range: (usize, usize), error_ranges: &[(usize, usize)]) -> bool {
        let (start, end) = range;
        error_ranges.iter().any(|&(err_start, err_end)| {
            !(end <= err_start || start >= err_end)
        })
    }

    /// Create a fallback `<file_level>` raw function symbol for degraded historical edits.
    pub fn create_file_level_symbol(file_line_count: u32, total_bytes: usize) -> RawFunctionSymbol {
        RawFunctionSymbol {
            name: "<file_level>".to_string(),
            qualname: "<file_level>".to_string(),
            signature: "<file_level_change>".to_string(),
            docstring: None,
            start_byte: 0,
            end_byte: total_bytes,
            start_line: 1,
            end_line: file_line_count.max(1),
            is_method: false,
        }
    }
}
