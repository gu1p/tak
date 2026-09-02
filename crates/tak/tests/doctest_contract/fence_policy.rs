use std::path::Path;

use super::Violation;

/// Evaluates one fenced code block against project doctest policy.
pub(crate) fn evaluate_closed_fence(
    path: &Path,
    line: usize,
    lang: &str,
    content: &str,
    found_valid_rust_fence: &mut bool,
    violations: &mut Vec<Violation>,
) {
    let normalized = if lang.is_empty() { "" } else { lang };

    if normalized == "ignore" {
        violations.push(Violation {
            file: path.to_path_buf(),
            line,
            message:
                "`ignore` fenced blocks are forbidden; use `no_run` or `compile_fail` with Reason:"
                    .to_string(),
        });
        return;
    }

    if matches!(normalized, "rust" | "no_run" | "compile_fail") {
        *found_valid_rust_fence = true;

        if matches!(normalized, "no_run" | "compile_fail") && !content.contains("Reason:") {
            violations.push(Violation {
                file: path.to_path_buf(),
                line,
                message: format!("`{normalized}` fenced block must include `Reason:` in the block"),
            });
        }
    }
}
