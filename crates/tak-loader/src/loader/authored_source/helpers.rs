use ruff_python_ast::Expr;
use ruff_text_size::{Ranged, TextRange};

use super::boundary::AuthoredDslBoundary;

impl<'a> AuthoredDslBoundary<'a> {
    pub(super) fn handle_call(&mut self, callee: &Expr) {
        if let Some(member_name) = namespace_method_name(callee, "Decision") {
            match member_name {
                "local" => {
                    self.allow_direct_decision_call(callee);
                    self.lower_attribute(callee, "Decision_local");
                }
                "remote" => {
                    self.allow_direct_decision_call(callee);
                    self.lower_attribute(callee, "Decision_remote");
                }
                "remote_any" => self.reject(
                    callee.range(),
                    "`Decision.remote_any(...)` is unsupported; use `Decision.remote(...)`.",
                ),
                _ => self.reject(
                    callee.range(),
                    format!(
                        "`Decision.{member_name}(...)` is unsupported; use `Decision.local(...)` or `Decision.remote(...)`."
                    ),
                ),
            }
        }

        if let Some(method_name) = namespace_method_name(callee, "RemoteTransportMode") {
            self.reject(
                callee.range(),
                format!(
                    "`RemoteTransportMode.{method_name}(...)` is unsupported; use `{method_name}()` instead."
                ),
            );
        }

        if namespace_method_name(callee, "ServiceAuth") == Some("from_env") {
            self.reject(
                callee.range(),
                "`ServiceAuth.from_env(...)` is unsupported; service auth helpers are not part of the shipped TASKS.py DSL.",
            );
        }
    }

    pub(super) fn handle_attribute(&mut self, expr: &Expr, range: TextRange) {
        if let Some(member_name) = namespace_attribute_name(expr, "Decision") {
            if self.is_allowed_decision_attribute(range) {
                return;
            }

            match member_name {
                "local" | "remote" => self.reject(
                    range,
                    format!(
                        "`Decision.{member_name}` may only be used as a direct call; use `Decision.{member_name}(...)`."
                    ),
                ),
                "remote_any" => self.reject(
                    range,
                    "`Decision.remote_any(...)` is unsupported; use `Decision.remote(...)`.",
                ),
                _ => self.reject(
                    range,
                    format!(
                        "`Decision.{member_name}` is unsupported; use `Decision.local(...)` or `Decision.remote(...)`."
                    ),
                ),
            }
        }

        if let Some(member_name) = namespace_attribute_name(expr, "WorkspaceTransferMode") {
            self.reject(
                range,
                format!(
                    "`WorkspaceTransferMode.{member_name}` is unsupported; use `{member_name}` instead."
                ),
            );
        }

        if let Some(member_name) = namespace_attribute_name(expr, "ResultSyncMode") {
            self.reject(
                range,
                format!(
                    "`ResultSyncMode.{member_name}` is unsupported; use `{member_name}` instead."
                ),
            );
        }
    }
}

fn namespace_method_name<'a>(expr: &'a Expr, namespace: &str) -> Option<&'a str> {
    let Expr::Attribute(attribute) = expr else {
        return None;
    };
    let Expr::Name(name) = attribute.value.as_ref() else {
        return None;
    };
    if name.id.as_str() != namespace {
        return None;
    }
    Some(attribute.attr.as_str())
}

fn namespace_attribute_name<'a>(expr: &'a Expr, namespace: &str) -> Option<&'a str> {
    let Expr::Attribute(attribute) = expr else {
        return None;
    };
    let Expr::Name(name) = attribute.value.as_ref() else {
        return None;
    };
    if name.id.as_str() != namespace {
        return None;
    }
    Some(attribute.attr.as_str())
}

pub(super) fn is_tak_module(module_name: &str) -> bool {
    module_name == "tak" || module_name.starts_with("tak.")
}

pub(super) fn line_and_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset];
    let line = prefix.chars().filter(|ch| *ch == '\n').count() + 1;
    let column = prefix.chars().rev().take_while(|ch| *ch != '\n').count() + 1;
    (line, column)
}
