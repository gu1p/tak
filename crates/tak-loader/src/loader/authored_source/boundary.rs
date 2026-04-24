use std::path::Path;

use anyhow::{Result, anyhow};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use super::{
    PreparedAuthoredSource,
    expr_helpers::{is_tak_module, line_and_column},
};

pub(super) struct AuthoredDslBoundary<'a> {
    path: &'a Path,
    source: &'a str,
    replacements: Vec<Replacement>,
    allowed_namespace_attribute_ranges: Vec<TextRange>,
    allowed_namespace_name_ranges: Vec<TextRange>,
    error: Option<String>,
}

struct Replacement {
    range: TextRange,
    replacement: &'static str,
}

impl<'a> AuthoredDslBoundary<'a> {
    pub(super) fn new(path: &'a Path, source: &'a str) -> Self {
        Self {
            path,
            source,
            replacements: Vec::new(),
            allowed_namespace_attribute_ranges: Vec::new(),
            allowed_namespace_name_ranges: Vec::new(),
            error: None,
        }
    }

    pub(super) fn finish(mut self) -> Result<PreparedAuthoredSource> {
        if let Some(message) = self.error.take() {
            return Err(anyhow!(message));
        }

        self.replacements
            .sort_by_key(|replacement| replacement.range.start());

        let mut runtime_source = self.source.to_owned();
        for replacement in self.replacements.into_iter().rev() {
            runtime_source.replace_range(
                replacement.range.start().to_usize()..replacement.range.end().to_usize(),
                replacement.replacement,
            );
        }

        Ok(PreparedAuthoredSource {
            authored_source: self.source.to_owned(),
            runtime_source,
        })
    }

    pub(super) fn reject(&mut self, range: TextRange, message: impl Into<String>) {
        if self.error.is_some() {
            return;
        }

        let (line, column) = line_and_column(self.source, range.start().to_usize());
        self.error = Some(format!(
            "{}:{}:{}: {}",
            self.path.display(),
            line,
            column,
            message.into()
        ));
    }

    pub(super) fn lower_attribute(&mut self, expr: &Expr, replacement: &'static str) {
        self.replacements.push(Replacement {
            range: expr.range(),
            replacement,
        });
    }

    pub(super) fn allow_namespace_attribute(&mut self, expr: &Expr) {
        self.allowed_namespace_attribute_ranges.push(expr.range());
        if let Expr::Attribute(attribute) = expr {
            self.allowed_namespace_name_ranges
                .push(attribute.value.as_ref().range());
        }
    }

    pub(super) fn is_allowed_namespace_attribute(&self, range: TextRange) -> bool {
        self.allowed_namespace_attribute_ranges.contains(&range)
    }

    pub(super) fn is_allowed_namespace_name(&self, range: TextRange) -> bool {
        self.allowed_namespace_name_ranges.contains(&range)
    }

    fn should_reject_name(&self, name: &str, range: TextRange) -> bool {
        is_namespace_name(name) && !self.is_allowed_namespace_name(range)
    }

    fn name_rejection_message(&self, name: &str) -> String {
        format!("`{name}` may only be used through the shipped TASKS.py namespace methods.")
    }
}

impl<'a> Visitor<'a> for AuthoredDslBoundary<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.error.is_none() {
            match stmt {
                Stmt::Import(import_stmt)
                    if import_stmt
                        .names
                        .iter()
                        .any(|alias| is_tak_module(alias.name.as_str())) =>
                {
                    self.reject(
                        stmt.range(),
                        "imports from `tak` are unsupported; use the shipped TASKS.py DSL directly.",
                    );
                }
                Stmt::ImportFrom(import_from)
                    if import_from
                        .module
                        .as_ref()
                        .is_some_and(|module| is_tak_module(module.as_str())) =>
                {
                    self.reject(
                        stmt.range(),
                        "imports from `tak` are unsupported; use the shipped TASKS.py DSL directly.",
                    );
                }
                _ => {}
            }
        }

        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.error.is_none() {
            match expr {
                Expr::Call(call) => self.handle_call(call.func.as_ref()),
                Expr::Attribute(attribute) => self.handle_attribute(expr, attribute.range()),
                Expr::Name(name) if self.should_reject_name(name.id.as_str(), name.range()) => {
                    self.reject(name.range(), self.name_rejection_message(name.id.as_str()));
                }
                _ => {}
            }
        }

        visitor::walk_expr(self, expr);
    }
}

fn is_namespace_name(name: &str) -> bool {
    matches!(
        name,
        "Decision"
            | "Execution"
            | "Runtime"
            | "Transport"
            | "SessionReuse"
            | "Scope"
            | "Hold"
            | "QueueDiscipline"
            | "SessionLifetime"
    )
}
