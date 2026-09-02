use std::path::Path;

use super::Violation;
use super::fence_policy::evaluate_closed_fence;

/// Validates that one function doc block contains compliant fenced Rust examples.
pub(crate) fn validate_doc_block(
    path: &Path,
    line: usize,
    docs: &[&str],
    violations: &mut Vec<Violation>,
) {
    let mut in_fence = false;
    let mut fence_lang = String::new();
    let mut fence_content = String::new();
    let mut found_valid_rust_fence = false;

    for raw_doc_line in docs {
        let text = raw_doc_line
            .trim_start()
            .strip_prefix("///")
            .expect("doc line should start with ///")
            .trim_start();

        if let Some(rest) = text.strip_prefix("```") {
            let token = rest
                .trim()
                .split(|ch: char| ch.is_whitespace() || ch == ',')
                .next()
                .unwrap_or("");

            if !in_fence {
                in_fence = true;
                fence_lang = token.to_string();
                fence_content.clear();
                continue;
            }

            evaluate_closed_fence(
                path,
                line,
                &fence_lang,
                &fence_content,
                &mut found_valid_rust_fence,
                violations,
            );
            in_fence = false;
            fence_lang.clear();
            fence_content.clear();
            continue;
        }

        if in_fence {
            fence_content.push_str(text);
            fence_content.push('\n');
        }
    }

    if in_fence {
        violations.push(Violation {
            file: path.to_path_buf(),
            line,
            message: "unterminated fenced code block in function docs".to_string(),
        });
    }

    if !found_valid_rust_fence {
        violations.push(Violation {
            file: path.to_path_buf(),
            line,
            message:
                "function docs must include at least one fenced Rust example (`rust`, `no_run`, or `compile_fail`)"
                    .to_string(),
        });
    }
}
