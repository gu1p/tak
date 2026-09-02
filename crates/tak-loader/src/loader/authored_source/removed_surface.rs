use ruff_python_ast::{Expr, ExprCall, ExprName};
use ruff_text_size::Ranged;

use super::{boundary::AuthoredDslBoundary, expr_helpers::namespace_method_name};

impl<'a> AuthoredDslBoundary<'a> {
    pub(super) fn reject_removed_call_surface(&mut self, call: &ExprCall) -> bool {
        self.reject_removed_queue_max_pending(call) || self.reject_removed_container_command(call)
    }

    fn reject_removed_queue_max_pending(&mut self, call: &ExprCall) -> bool {
        let Expr::Name(ExprName { id, .. }) = call.func.as_ref() else {
            return false;
        };
        if id.as_str() != "queue_def" {
            return false;
        }
        let Some(keyword) = call.arguments.keywords.iter().find(|keyword| {
            keyword
                .arg
                .as_ref()
                .is_some_and(|name| name.as_str() == "max_pending")
        }) else {
            return false;
        };
        self.reject(
            keyword.range(),
            "queue_def `max_pending` was removed in spec_version=2; use `slots`.",
        );
        true
    }

    fn reject_removed_container_command(&mut self, call: &ExprCall) -> bool {
        if !matches!(
            namespace_method_name(call.func.as_ref(), "Container"),
            Some("Image" | "Dockerfile")
        ) {
            return false;
        }
        let Some(keyword) = call.arguments.keywords.iter().find(|keyword| {
            keyword
                .arg
                .as_ref()
                .is_some_and(|name| name.as_str() == "command")
        }) else {
            return false;
        };
        self.reject(
            keyword.range(),
            "Container `command` was removed in spec_version=2; use task steps (`cmd(...)` or `script(...)`).",
        );
        true
    }

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
