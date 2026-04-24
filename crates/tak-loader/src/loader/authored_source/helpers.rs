use ruff_python_ast::Expr;
use ruff_text_size::{Ranged, TextRange};

use super::{
    boundary::AuthoredDslBoundary,
    expr_helpers::{namespace_attribute_name, namespace_method_name},
    replacements::*,
};

impl<'a> AuthoredDslBoundary<'a> {
    pub(super) fn handle_call(&mut self, callee: &Expr) {
        if let Some(member_name) = namespace_method_name(callee, "Decision") {
            match member_name {
                "local" => {
                    self.allow_namespace_attribute(callee);
                    self.lower_attribute(callee, "Decision_local");
                }
                "remote" => {
                    self.allow_namespace_attribute(callee);
                    self.lower_attribute(callee, "Decision_remote");
                }
                _ => self.reject(
                    callee.range(),
                    format!(
                        "`Decision.{member_name}(...)` is unsupported; use `Decision.local(...)` or `Decision.remote(...)`."
                    ),
                ),
            }
        }

        if self.lower_namespace_call(callee, "Execution", execution_method_replacement) {
            return;
        }
        if self.lower_namespace_call(callee, "Runtime", runtime_method_replacement) {
            return;
        }
        if self.lower_namespace_call(callee, "Transport", transport_method_replacement) {
            return;
        }
        if self.lower_namespace_call(callee, "SessionReuse", session_reuse_method_replacement) {
            return;
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
            if self.is_allowed_namespace_attribute(range) {
                return;
            }

            match member_name {
                "local" | "remote" => self.reject(
                    range,
                    format!(
                    "`Decision.{member_name}` may only be used as a direct call; use `Decision.{member_name}(...)`."
                    ),
                ),
                _ => self.reject(
                    range,
                    format!(
                        "`Decision.{member_name}` is unsupported; use `Decision.local(...)` or `Decision.remote(...)`."
                    ),
                ),
            }
        }

        if self.lower_namespace_constant(expr, "Scope", scope_constant_replacement) {
            return;
        }
        if self.lower_namespace_constant(expr, "Hold", hold_constant_replacement) {
            return;
        }
        if self.lower_namespace_constant(
            expr,
            "QueueDiscipline",
            queue_discipline_constant_replacement,
        ) {
            return;
        }
        if self.lower_namespace_constant(
            expr,
            "SessionLifetime",
            session_lifetime_constant_replacement,
        ) {
            return;
        }

        for namespace in ["Execution", "Runtime", "Transport", "SessionReuse"] {
            if let Some(member_name) = namespace_attribute_name(expr, namespace) {
                if self.is_allowed_namespace_attribute(range) {
                    return;
                }
                self.reject(
                    range,
                    format!(
                        "`{namespace}.{member_name}` may only be used as a direct call; use `{namespace}.{member_name}(...)`."
                    ),
                );
                return;
            }
        }
    }

    fn lower_namespace_call(
        &mut self,
        callee: &Expr,
        namespace: &str,
        replacement: fn(&str) -> Option<&'static str>,
    ) -> bool {
        let Some(member_name) = namespace_method_name(callee, namespace) else {
            return false;
        };
        let Some(replacement) = replacement(member_name) else {
            self.reject(
                callee.range(),
                format!("`{namespace}.{member_name}(...)` is unsupported."),
            );
            return true;
        };
        self.allow_namespace_attribute(callee);
        self.lower_attribute(callee, replacement);
        true
    }

    fn lower_namespace_constant(
        &mut self,
        expr: &Expr,
        namespace: &str,
        replacement: fn(&str) -> Option<&'static str>,
    ) -> bool {
        let Some(member_name) = namespace_attribute_name(expr, namespace) else {
            return false;
        };
        let Some(replacement) = replacement(member_name) else {
            self.reject(
                expr.range(),
                format!("`{namespace}.{member_name}` is unsupported."),
            );
            return true;
        };
        self.allow_namespace_attribute(expr);
        self.lower_attribute(expr, replacement);
        true
    }
}
