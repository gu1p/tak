use std::path::Path;

use anyhow::{Result, anyhow};
use ruff_python_ast::ModModule;
use ruff_python_ast::visitor::Visitor;
use ruff_python_parser::parse_module;
use ruff_text_size::TextRange;

mod boundary;
mod expr_helpers;
mod helpers;
mod module_declaration;
mod namespace_lowering;
mod removed_surface;
mod replacements;
mod spec_version;

#[cfg(test)]
mod module_declaration_shape_tests;
#[cfg(test)]
mod module_declaration_version_tests;
pub(crate) use module_declaration::{ModuleDeclaration, SpecVersionMarker};
pub(crate) use spec_version::{AuthoredSpecVersion, classify_authored_version};

pub(crate) struct PreparedAuthoredSource {
    pub(crate) authored_source: String,
    pub(crate) runtime_source: String,
}

pub(crate) struct ParsedAuthoredSource<'a> {
    path: &'a Path,
    source: &'a str,
    syntax: ModModule,
}

impl<'a> ParsedAuthoredSource<'a> {
    pub(crate) fn parse(path: &'a Path, source: &'a str) -> Result<Self> {
        let syntax = parse_module(source)
            .map_err(|err| {
                anyhow!(
                    "failed to parse {} for TASKS.py DSL validation: {err}",
                    path.display()
                )
            })?
            .into_syntax();
        Ok(Self {
            path,
            source,
            syntax,
        })
    }

    pub(crate) fn module_declaration(&self) -> Result<Option<ModuleDeclaration>> {
        module_declaration::find(self.path, self.source, &self.syntax.body)
    }

    pub(crate) fn prepare_v2(&self) -> Result<PreparedAuthoredSource> {
        let mut boundary = boundary::AuthoredDslBoundary::new(self.path, self.source);
        boundary.visit_body(&self.syntax.body);
        boundary.finish()
    }
}

pub(crate) fn classify_source(path: &Path, source: &str) -> Result<AuthoredSpecVersion> {
    let parsed = ParsedAuthoredSource::parse(path, source)?;
    let declaration = parsed.module_declaration()?;
    classify_authored_version(path, source, declaration.as_ref())
}

pub(crate) fn prepare_v2_authored_source(
    path: &Path,
    source: &str,
) -> Result<PreparedAuthoredSource> {
    let parsed = ParsedAuthoredSource::parse(path, source)?;
    let declaration = parsed.module_declaration()?;
    if classify_authored_version(path, source, declaration.as_ref())? != AuthoredSpecVersion::V2 {
        return Err(anyhow!(
            "{}: module_spec(spec_version=2) is required for v2 evaluation",
            path.display()
        ));
    }
    parsed.prepare_v2()
}

pub(crate) fn authored_error(
    path: &Path,
    source: &str,
    range: TextRange,
    message: impl Into<String>,
) -> anyhow::Error {
    let (line, column) = expr_helpers::line_and_column(source, range.start().to_usize());
    anyhow!("{}:{}:{}: {}", path.display(), line, column, message.into())
}
