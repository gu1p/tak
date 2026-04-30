use ruff_python_ast::{Expr, ExprName};

use super::boundary::AuthoredDslBoundary;

impl<'a> AuthoredDslBoundary<'a> {
    pub(super) fn reject_removed_function_call(&mut self, callee: &Expr) -> bool {
        let Expr::Name(ExprName { id, range, .. }) = callee else {
            return false;
        };
        if id.as_str() != "execution_policy" {
            return false;
        }
        self.reject(
            *range,
            "`execution_policy(...)` was replaced; use `Execution.FirstAvailable([...])`.",
        );
        true
    }
}
