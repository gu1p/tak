use std::path::Path;

use anyhow::{Result, anyhow};
use monty::MontyObject;
use ruff_python_ast::visitor::Visitor;
use ruff_python_parser::parse_module;

mod boundary;
mod helpers;
mod reason_namespace;

pub(crate) struct PreparedAuthoredSource {
    pub(crate) authored_source: String,
    pub(crate) runtime_source: String,
}

pub(crate) fn prepare_authored_source(path: &Path, source: &str) -> Result<PreparedAuthoredSource> {
    let parsed = parse_module(source).map_err(|err| {
        anyhow!(
            "failed to parse {} for TASKS.py DSL validation: {err}",
            path.display()
        )
    })?;

    let mut boundary = boundary::AuthoredDslBoundary::new(path, source);
    boundary.visit_body(&parsed.syntax().body);
    boundary.finish()
}

pub(crate) fn runtime_input_names() -> Vec<String> {
    vec!["Reason".to_owned()]
}

pub(crate) fn runtime_inputs() -> Vec<MontyObject> {
    vec![reason_namespace::reason_namespace()]
}
