use anyhow::{Result, anyhow};
use ruff_python_ast::{Stmt, StmtAnnAssign, StmtClassDef};
use ruff_python_parser::parse_module;
use ruff_text_size::Ranged;

#[path = "dsl_ast.rs"]
mod dsl_ast;

use self::dsl_ast::{
    SourceLines, class_start_range, expr_name, extract_python_docstrings, function_start_range,
};

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
    pub(super) summary: String,
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
    let parsed = parse_module(DSL_STUBS)
        .map_err(|err| anyhow!("failed to parse embedded TASKS.py DSL stubs: {err}"))?;
    let source = SourceLines::new(DSL_STUBS);
    let mut docs = DslDocs::default();

    for statement in &parsed.syntax().body {
        match statement {
            Stmt::ClassDef(class_def) => {
                let (entry, methods) = collect_class_docs(class_def, &source);
                if !entry.signature.contains("TypedDict") {
                    for field in &entry.fields {
                        docs.constants
                            .push(class_field_constant(&entry.name, field));
                    }
                }
                docs.types.push(entry);
                docs.methods.extend(methods);
            }
            Stmt::FunctionDef(function) => {
                let name = function.name.as_str().to_owned();
                let stub_summary = source.comments_before(function_start_range(function));
                let summary = docstrings.get(&name).cloned().unwrap_or(stub_summary);
                docs.functions.push(DslFunctionEntry {
                    name,
                    signature: source.function_signature(function),
                    summary,
                });
            }
            Stmt::AnnAssign(assign) => {
                if let Some(entry) = collect_constant_doc(assign, &source) {
                    docs.constants.push(entry);
                }
            }
            _ => {}
        }
    }

    Ok(docs)
}

fn collect_class_docs(
    class_def: &StmtClassDef,
    source: &SourceLines<'_>,
) -> (DslTypeEntry, Vec<DslMethodEntry>) {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    let class_name = class_def.name.as_str().to_owned();

    for statement in &class_def.body {
        match statement {
            Stmt::AnnAssign(assign) => {
                if let Some(field) = collect_field_doc(assign, source) {
                    fields.push(field);
                }
            }
            Stmt::FunctionDef(function) => {
                methods.push(DslMethodEntry {
                    owner: class_name.clone(),
                    name: function.name.as_str().to_owned(),
                    signature: source.function_signature(function),
                    summary: source.comments_before(function_start_range(function)),
                });
            }
            _ => {}
        }
    }

    (
        DslTypeEntry {
            name: class_name,
            signature: source.class_signature(class_def),
            summary: source.comments_before(class_start_range(class_def)),
            fields,
        },
        methods,
    )
}

fn collect_field_doc(assign: &StmtAnnAssign, source: &SourceLines<'_>) -> Option<DslFieldEntry> {
    let name = expr_name(assign.target.as_ref())?;
    Some(DslFieldEntry {
        name,
        ty: source.annotation_source(assign.annotation.range()),
        summary: source.comments_before(assign.range()),
    })
}

fn collect_constant_doc(
    assign: &StmtAnnAssign,
    source: &SourceLines<'_>,
) -> Option<DslConstantEntry> {
    let name = expr_name(assign.target.as_ref())?;
    let ty = source.annotation_source(assign.annotation.range());
    let summary = source.comments_before(assign.range());
    Some(DslConstantEntry {
        name,
        signature: source.inline_source(assign.range()),
        summary: if summary.is_empty() {
            format!("Typed constant with value type `{ty}`.")
        } else {
            summary
        },
    })
}

fn class_field_constant(owner: &str, field: &DslFieldEntry) -> DslConstantEntry {
    DslConstantEntry {
        name: format!("{owner}.{}", field.name),
        signature: format!("{owner}.{}: {}", field.name, field.ty),
        summary: if field.summary.is_empty() {
            format!("Typed constant with value type `{}`.", field.ty)
        } else {
            field.summary.clone()
        },
    }
}
