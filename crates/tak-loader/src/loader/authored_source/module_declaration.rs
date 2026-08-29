use std::path::Path;

use anyhow::Result;
use ruff_python_ast::{Expr, ExprCall, Number, Stmt};
use ruff_text_size::{Ranged, TextRange};

use super::authored_error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpecVersionMarker {
    Omitted,
    Literal(u32),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ModuleDeclaration {
    pub(crate) version: SpecVersionMarker,
    pub(crate) marker_range: TextRange,
}

pub(super) fn find(path: &Path, source: &str, body: &[Stmt]) -> Result<Option<ModuleDeclaration>> {
    let mut declaration = None;
    for statement in body {
        let Some(call) = direct_module_spec_call(statement) else {
            continue;
        };
        if declaration.is_some() {
            return Err(authored_error(
                path,
                source,
                call.range(),
                "found more than one module_spec declaration; declare exactly one top-level module_spec call",
            ));
        }
        declaration = Some(classify_call(path, source, call)?);
    }
    Ok(declaration)
}

fn direct_module_spec_call(statement: &Stmt) -> Option<&ExprCall> {
    let value = match statement {
        Stmt::Assign(assign) => assign.value.as_ref(),
        Stmt::AnnAssign(assign) => assign.value.as_deref()?,
        Stmt::Expr(expression) => expression.value.as_ref(),
        _ => return None,
    };
    let Expr::Call(call) = value else {
        return None;
    };
    let Expr::Name(name) = call.func.as_ref() else {
        return None;
    };
    (name.id.as_str() == "module_spec").then_some(call)
}

fn classify_call(path: &Path, source: &str, call: &ExprCall) -> Result<ModuleDeclaration> {
    let mut marker = None;
    let mut expansion = None;
    for keyword in &call.arguments.keywords {
        match keyword.arg.as_ref().map(|name| name.as_str()) {
            Some("spec_version") if marker.is_some() => {
                return Err(authored_error(
                    path,
                    source,
                    keyword.range(),
                    "spec_version may be declared only once",
                ));
            }
            Some("spec_version") => marker = Some(keyword),
            None => expansion = Some(keyword.range()),
            _ => {}
        }
    }

    let Some(keyword) = marker else {
        if let Some(range) = expansion {
            return Err(keyword_expansion_error(path, source, range));
        }
        return Ok(ModuleDeclaration {
            version: SpecVersionMarker::Omitted,
            marker_range: call.func.range(),
        });
    };
    let Expr::NumberLiteral(number) = &keyword.value else {
        return Err(non_literal_error(path, source, keyword.value.range()));
    };
    let Number::Int(value) = &number.value else {
        return Err(non_literal_error(path, source, keyword.value.range()));
    };
    let Some(value) = value.as_u32() else {
        return Err(non_literal_error(path, source, keyword.value.range()));
    };
    Ok(ModuleDeclaration {
        version: SpecVersionMarker::Literal(value),
        marker_range: keyword.value.range(),
    })
}

fn non_literal_error(path: &Path, source: &str, range: TextRange) -> anyhow::Error {
    authored_error(
        path,
        source,
        range,
        "spec_version must be the integer literal 2 exactly",
    )
}

fn keyword_expansion_error(path: &Path, source: &str, range: TextRange) -> anyhow::Error {
    authored_error(
        path,
        source,
        range,
        "module_spec keyword expansion cannot establish an authored spec_version; declare spec_version=2 explicitly as an integer literal",
    )
}
