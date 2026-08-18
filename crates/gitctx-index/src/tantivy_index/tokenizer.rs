use tantivy::tokenizer::{Token, TokenStream, Tokenizer};

/// Tokenizer for source code identifiers and expressions.
///
/// Implements camelCase, PascalCase, snake_case splitting, namespace/path segment extraction,
/// and preserves identifier integrity without NLP stemming per D-05 and INDEX-03.
#[derive(Clone, Debug, Default)]
pub struct CodeTokenizer;

impl Tokenizer for CodeTokenizer {
    type TokenStream<'a> = CodeTokenStream;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        let tokens = tokenize_code(text);
        CodeTokenStream { tokens, index: 0 }
    }
}

pub struct CodeTokenStream {
    tokens: Vec<Token>,
    index: usize,
}

impl TokenStream for CodeTokenStream {
    fn advance(&mut self) -> bool {
        if self.index < self.tokens.len() {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn token(&self) -> &Token {
        &self.tokens[self.index - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.index - 1]
    }
}

/// Tokenize code text into Tantivy tokens.
pub fn tokenize_code(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut position = 0;

    let len = text.len();
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < len {
        // Skip whitespace and standard punctuation that are not part of identifiers or paths
        while i < len && is_delimiter(bytes[i]) {
            i += 1;
        }

        if i >= len {
            break;
        }

        let start = i;
        while i < len && !is_delimiter(bytes[i]) {
            i += 1;
        }
        let end = i;

        if start < end {
            let chunk = &text[start..end];
            emit_chunk_tokens(chunk, start, &mut tokens, &mut position);
        }
    }

    tokens
}

fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b' ' | b'\t'
            | b'\r'
            | b'\n'
            | b'('
            | b')'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'<'
            | b'>'
            | b','
            | b';'
            | b'"'
            | b'\''
            | b'`'
            | b'~'
            | b'!'
            | b'@'
            | b'#'
            | b'%'
            | b'^'
            | b'&'
            | b'*'
            | b'+'
            | b'='
            | b'|'
            | b'?'
    )
}

fn emit_chunk_tokens(
    chunk: &str,
    chunk_offset: usize,
    tokens: &mut Vec<Token>,
    position: &mut usize,
) {
    let mut seen_texts = std::collections::HashSet::new();

    let mut push_token = |text: &str, start_offset: usize, end_offset: usize| {
        let trimmed = text.trim();
        if !trimmed.is_empty() && seen_texts.insert(trimmed.to_string()) {
            tokens.push(Token {
                offset_from: start_offset,
                offset_to: end_offset,
                position: *position,
                text: trimmed.to_string(),
                position_length: 1,
            });
            *position += 1;
        }
    };

    // 1. Emit verbatim full chunk
    push_token(chunk, chunk_offset, chunk_offset + chunk.len());

    // 2. Emit lowercased full chunk if different
    let lower_chunk = chunk.to_lowercase();
    if lower_chunk != chunk {
        push_token(&lower_chunk, chunk_offset, chunk_offset + chunk.len());
    }

    // 3. Split by path/namespace separators ("::", "/", "\\", ".", "-", "$", "->", "_")
    let segments = split_by_separators(chunk);
    for (seg_start, seg_end, seg_str) in segments {
        let abs_start = chunk_offset + seg_start;
        let abs_end = chunk_offset + seg_end;

        // Emit segment verbatim & lowercased
        push_token(seg_str, abs_start, abs_end);
        let lower_seg = seg_str.to_lowercase();
        if lower_seg != seg_str {
            push_token(&lower_seg, abs_start, abs_end);
        }

        // 4. Split segment by camelCase / PascalCase / snake_case / numbers
        let subtokens = split_subtokens(seg_str);
        for (sub_start, sub_end, sub_str) in subtokens {
            let sub_abs_start = abs_start + sub_start;
            let sub_abs_end = abs_start + sub_end;
            let sub_lower = sub_str.to_lowercase();
            push_token(&sub_lower, sub_abs_start, sub_abs_end);
        }
    }
}

fn split_by_separators(s: &str) -> Vec<(usize, usize, &str)> {
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut start = 0;
    let mut i = 0;

    while i < len {
        // Check for "::" or "->"
        let is_sep_2 = (i + 1 < len)
            && ((bytes[i] == b':' && bytes[i + 1] == b':')
                || (bytes[i] == b'-' && bytes[i + 1] == b'>'));

        let is_sep_1 = matches!(bytes[i], b'/' | b'\\' | b'.' | b'-' | b'$' | b'_' | b':');

        if is_sep_2 {
            if i > start {
                result.push((start, i, &s[start..i]));
            }
            i += 2;
            start = i;
        } else if is_sep_1 {
            if i > start {
                result.push((start, i, &s[start..i]));
            }
            i += 1;
            start = i;
        } else {
            i += 1;
        }
    }

    if start < len {
        result.push((start, len, &s[start..len]));
    }

    result
}

fn split_subtokens(s: &str) -> Vec<(usize, usize, &str)> {
    let mut result = Vec::new();
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    if chars.is_empty() {
        return result;
    }

    let mut start_idx = 0;
    for i in 0..chars.len() {
        let curr = chars[i].1;
        let next = chars.get(i + 1).map(|(_, c)| *c);
        let next_next = chars.get(i + 2).map(|(_, c)| *c);

        let mut is_boundary = false;

        if let Some(next_c) = next {
            // camelCase boundary: lower followed by upper (e.g. 'r' in "getUser" -> boundary between 'r' and 'U')
            if curr.is_lowercase() && next_c.is_uppercase() {
                is_boundary = true;
            }
            // Acronym boundary: uppercase followed by uppercase and then lowercase (e.g. 'P' in "HTTPServer")
            else if curr.is_uppercase() && next_c.is_uppercase() {
                if let Some(nn_c) = next_next {
                    if nn_c.is_lowercase() {
                        is_boundary = true;
                    }
                }
            }
            // Letter to digit or digit to letter
            else if (curr.is_alphabetic() && next_c.is_numeric())
                || (curr.is_numeric() && next_c.is_alphabetic())
            {
                is_boundary = true;
            }
        }

        if is_boundary || i + 1 == chars.len() {
            let start_byte = chars[start_idx].0;
            let end_byte = if is_boundary { chars[i + 1].0 } else { s.len() };
            if start_byte < end_byte {
                result.push((start_byte, end_byte, &s[start_byte..end_byte]));
            }
            start_idx = i + 1;
        }
    }

    result
}
