use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use super::{
    boundary::AuthoredDslBoundary,
    expr_helpers::{namespace_attribute_name, namespace_method_name},
};

impl<'a> AuthoredDslBoundary<'a> {
    pub(super) fn lower_namespace_call(
        &mut self,
        callee: &Expr,
        namespace: &str,
        replacement: fn(&str) -> Option<&'static str>,
    ) -> bool {
        let Some(member_name) = namespace_method_name(callee, namespace) else {
            return false;
        };
        let Some(replacement) = replacement(member_name) else {
            if namespace == "Execution" && member_name == "Policy" {
                self.reject(
                    callee.range(),
                    "`Execution.Policy(...)` is unsupported; use `Execution.Decide(...)`.",
                );
                return true;
            }
            if namespace == "Execution" && member_name == "Session" {
                self.reject(
                    callee.range(),
                    "`Execution.Session(...)` was replaced; use `task(..., use_session=SESSION)`.",
                );
                return true;
            }
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

    pub(super) fn lower_namespace_constant(
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
