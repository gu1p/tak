use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use ruff_python_ast::{Expr, Stmt, StmtClassDef, StmtFunctionDef};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange};

use super::super::text::normalize_doc_text;

pub(super) fn extract_python_docstrings(source: &str) -> Result<BTreeMap<String, String>> {
    let parsed = parse_module(source)
        .map_err(|err| anyhow!("failed to parse embedded TASKS.py prelude: {err}"))?;
    let mut docs = BTreeMap::new();
    for statement in &parsed.syntax().body {
        if let Stmt::FunctionDef(function) = statement
            && let Some(docstring) = function_docstring(function)
        {
            docs.insert(function.name.as_str().to_owned(), docstring);
        }
    }
    Ok(docs)
}

pub(super) fn expr_name(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Name(name) => Some(name.id.as_str().to_owned()),
        _ => None,
    }
}

pub(super) fn class_start_range(class_def: &StmtClassDef) -> TextRange {
    class_def
        .decorator_list
        .first()
        .map(Ranged::range)
        .unwrap_or_else(|| class_def.range())
}

pub(super) fn function_start_range(function: &StmtFunctionDef) -> TextRange {
    function
        .decorator_list
        .first()
        .map(Ranged::range)
        .unwrap_or_else(|| function.range())
}

pub(super) struct SourceLines<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> SourceLines<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (offset, character) in source.char_indices() {
            if character == '\n' {
                line_starts.push(offset + 1);
            }
        }
        Self {
            source,
            line_starts,
        }
    }

    pub(super) fn class_signature(&self, class_def: &StmtClassDef) -> String {
        let source = self.source_at(class_def.range());
        let start = source.find("class ").unwrap_or_default();
        let header = &source[start..];
        let end = header
            .find(':')
            .map(|offset| offset + 1)
            .unwrap_or(header.len());
        normalize_inline_source(&header[..end])
    }

    pub(super) fn function_signature(&self, function: &StmtFunctionDef) -> String {
        let source = self.source_at(function.range());
        let start = source.find("def ").unwrap_or_default();
        let signature = &source[start..];
        let end = signature
            .find(": ...")
            .map(|offset| offset + ": ...".len())
            .unwrap_or(signature.len());
        normalize_inline_source(&signature[..end])
    }

    pub(super) fn inline_source(&self, range: TextRange) -> String {
        normalize_inline_source(self.source_at(range))
    }

    pub(super) fn annotation_source(&self, range: TextRange) -> String {
        let (start, end) = self.annotation_bounds(range);
        normalize_inline_source(&self.source[start..end])
    }

    pub(super) fn comments_before(&self, range: TextRange) -> String {
        let mut comments = Vec::new();
        let mut index = self.line_index_at(range.start().to_usize());
        while index > 0 {
            index -= 1;
            let Some(line) = self.line(index) else {
                break;
            };
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            let Some(comment) = trimmed.strip_prefix('#') else {
                break;
            };
            comments.push(comment.trim_start().to_owned());
        }
        comments.reverse();
        normalize_doc_text(&comments.join("\n"))
    }

    fn annotation_bounds(&self, range: TextRange) -> (usize, usize) {
        let mut start = range.start().to_usize();
        let mut end = range.end().to_usize();
        let before = &self.source[..start];
        let open = before.bytes().rposition(|byte| !byte.is_ascii_whitespace());
        let Some(open) = open.filter(|offset| before.as_bytes()[*offset] == b'(') else {
            return (start, end);
        };

        let after = &self.source[end..];
        let close = after.bytes().position(|byte| !byte.is_ascii_whitespace());
        let Some(close) = close.filter(|offset| after.as_bytes()[*offset] == b')') else {
            return (start, end);
        };

        start = open;
        end += close + 1;
        (start, end)
    }

    fn source_at(&self, range: TextRange) -> &'a str {
        &self.source[range.start().to_usize()..range.end().to_usize()]
    }

    fn line_index_at(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
    }

    fn line(&self, index: usize) -> Option<&'a str> {
        let start = *self.line_starts.get(index)?;
        let end = self
            .line_starts
            .get(index + 1)
            .map(|next| next.saturating_sub(1))
            .unwrap_or(self.source.len());
        Some(self.source[start..end].trim_end_matches('\r'))
    }
}

fn function_docstring(function: &StmtFunctionDef) -> Option<String> {
    let Some(Stmt::Expr(statement)) = function.body.first() else {
        return None;
    };
    let Expr::StringLiteral(literal) = statement.value.as_ref() else {
        return None;
    };
    Some(normalize_doc_text(literal.value.to_str()))
}

fn normalize_inline_source(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}
