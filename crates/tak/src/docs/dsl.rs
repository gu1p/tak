use std::collections::BTreeMap;

use anyhow::{Result, anyhow, bail};

use super::text::normalize_doc_text;

const PRELUDE: &str = include_str!("../../../tak-loader/src/loader/prelude.py");
const DSL_STUBS: &str = include_str!("../../../tak-loader/src/loader/dsl_stubs.pyi");

#[derive(Debug, Default)]
pub(super) struct DslDocs {
    pub(super) types: Vec<DslTypeEntry>,
    pub(super) constants: Vec<DslConstantEntry>,
    pub(super) functions: Vec<DslFunctionEntry>,
    pub(super) methods: Vec<DslMethodEntry>,
}

#[derive(Debug)]
pub(super) struct DslTypeEntry {
    pub(super) name: String,
    pub(super) signature: String,
    pub(super) summary: String,
    pub(super) fields: Vec<DslFieldEntry>,
}

#[derive(Debug)]
pub(super) struct DslFieldEntry {
    pub(super) name: String,
    pub(super) ty: String,
}

#[derive(Debug)]
pub(super) struct DslConstantEntry {
    pub(super) name: String,
    pub(super) signature: String,
    pub(super) summary: String,
}

#[derive(Debug)]
pub(super) struct DslFunctionEntry {
    pub(super) name: String,
    pub(super) signature: String,
    pub(super) summary: String,
}

#[derive(Debug)]
pub(super) struct DslMethodEntry {
    pub(super) owner: String,
    pub(super) name: String,
    pub(super) signature: String,
    pub(super) summary: String,
}

pub(super) fn collect_dsl_docs() -> Result<DslDocs> {
    let docstrings = extract_python_docstrings(PRELUDE)?;
    let lines = DSL_STUBS.lines().collect::<Vec<_>>();
    let mut docs = DslDocs::default();
    let mut pending_comments = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let raw_line = lines[index];
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            pending_comments.clear();
            index += 1;
            continue;
        }

        if !is_top_level(raw_line) {
            index += 1;
            continue;
        }

        if let Some(comment) = parse_stub_comment(trimmed) {
            pending_comments.push(comment.to_string());
            index += 1;
            continue;
        }

        if trimmed.starts_with("from ") || trimmed.starts_with("import ") {
            pending_comments.clear();
            index += 1;
            continue;
        }

        if trimmed.starts_with("class ") {
            let summary = consume_pending_comments(&mut pending_comments);
            let (entry, methods, next_index) = parse_typed_dict_class(&lines, index, summary)?;
            docs.types.push(entry);
            docs.methods.extend(methods);
            index = next_index;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("def ") {
            let Some(name_end) = rest.find('(') else {
                bail!("failed to parse Python function name from `{trimmed}`");
            };
            let name = rest[..name_end].trim().to_string();
            let stub_summary = consume_pending_comments(&mut pending_comments);
            let (signature, next_index) = parse_function_signature(&lines, index);
            let summary = docstrings.get(&name).cloned().unwrap_or(stub_summary);
            docs.functions.push(DslFunctionEntry {
                name,
                signature,
                summary,
            });
            index = next_index;
            continue;
        }

        if let Some((name, ty)) = parse_annotated_name_and_type(trimmed) {
            let summary = consume_pending_comments(&mut pending_comments);
            docs.constants.push(DslConstantEntry {
                name,
                signature: trimmed.to_string(),
                summary: if summary.is_empty() {
                    format!("Typed constant with value type `{ty}`.")
                } else {
                    summary
                },
            });
            index += 1;
            continue;
        }

        pending_comments.clear();
        index += 1;
    }

    Ok(docs)
}

include!("dsl_parse.rs");
