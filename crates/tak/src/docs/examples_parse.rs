use super::text::normalize_doc_text;

pub(super) fn extract_parenthesized_body(source: &str) -> Option<(&str, usize)> {
    let mut depth = 1usize;
    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = advance_over_python_string(source, index)?;
            }
            b'(' => {
                depth += 1;
                index += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&source[..index], index + 1));
                }
                index += 1;
            }
            _ => index += 1,
        }
    }

    None
}

pub(super) fn extract_first_python_string(source: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                return parse_python_string_literal(source, index).map(|(value, _)| value);
            }
            b'#' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }

    None
}

pub(super) fn extract_keyword_string(source: &str, keyword: &str) -> Option<String> {
    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\'' | b'"' => {
                index = advance_over_python_string(source, index)?;
            }
            b'#' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            _ if is_identifier_start(bytes[index]) => {
                let start = index;
                index += 1;
                while index < bytes.len() && is_identifier_continue(bytes[index]) {
                    index += 1;
                }
                if &source[start..index] != keyword {
                    continue;
                }

                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                if bytes.get(index) != Some(&b'=') {
                    continue;
                }
                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                if matches!(bytes.get(index), Some(b'\'') | Some(b'"')) {
                    return parse_python_string_literal(source, index).map(|(value, _)| value);
                }
            }
            _ => index += 1,
        }
    }

    None
}

fn advance_over_python_string(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(start)?;
    let is_triple = bytes.get(start + 1) == Some(&quote) && bytes.get(start + 2) == Some(&quote);
    let mut index = if is_triple { start + 3 } else { start + 1 };

    while index < bytes.len() {
        if is_triple {
            if bytes.get(index) == Some(&quote)
                && bytes.get(index + 1) == Some(&quote)
                && bytes.get(index + 2) == Some(&quote)
            {
                return Some(index + 3);
            }
            index += 1;
            continue;
        }

        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }
        if bytes[index] == quote {
            return Some(index + 1);
        }
        index += 1;
    }

    None
}

fn parse_python_string_literal(source: &str, start: usize) -> Option<(String, usize)> {
    let bytes = source.as_bytes();
    let quote = *bytes.get(start)?;
    let is_triple = bytes.get(start + 1) == Some(&quote) && bytes.get(start + 2) == Some(&quote);
    let mut index = if is_triple { start + 3 } else { start + 1 };
    let content_start = index;

    while index < bytes.len() {
        if is_triple {
            if bytes.get(index) == Some(&quote)
                && bytes.get(index + 1) == Some(&quote)
                && bytes.get(index + 2) == Some(&quote)
            {
                let value = normalize_doc_text(&source[content_start..index]);
                return Some((value, index + 3));
            }
            index += 1;
            continue;
        }

        if bytes[index] == b'\\' {
            index += 2;
            continue;
        }
        if bytes[index] == quote {
            let value = normalize_doc_text(&source[content_start..index]);
            return Some((value, index + 1));
        }
        index += 1;
    }

    None
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}
