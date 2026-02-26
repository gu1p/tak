//! Contract tests that enforce doctest coverage and policy for source function docs.

use std::fs;
use std::path::{Path, PathBuf};

/// One docs policy violation discovered by the contract scanner.
#[derive(Debug)]
struct Violation {
    file: PathBuf,
    line: usize,
    message: String,
}

/// Enforces strict function-doc doctest policy across all crate `src/` files.
#[test]
fn function_docs_include_doctest_blocks() {
    let repo_root = repo_root();
    let mut source_files = Vec::new();
    collect_rust_source_files(&repo_root.join("crates"), &mut source_files);

    let mut violations = Vec::new();
    for file in &source_files {
        validate_file_docs(file, &mut violations);
    }

    if !violations.is_empty() {
        let mut message = String::from("doc policy violations found:\n");
        for violation in violations {
            message.push_str(&format!(
                "- {}:{}: {}\n",
                violation.file.display(),
                violation.line,
                violation.message
            ));
        }
        panic!("{message}");
    }
}

/// Resolves repository root from the taskcraft crate's manifest directory.
fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("repository root should be two levels above taskcraft crate")
        .to_path_buf()
}

/// Recursively collects `src/*.rs` files under the provided directory.
fn collect_rust_source_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("failed to read directory {}: {err}", dir.display()));

    for entry in entries {
        let entry = entry.unwrap_or_else(|err| {
            panic!("failed to read directory entry in {}: {err}", dir.display())
        });
        let path = entry.path();

        if path.is_dir() {
            collect_rust_source_files(&path, files);
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        if !path
            .components()
            .any(|component| component.as_os_str() == "src")
        {
            continue;
        }

        files.push(path);
    }
}

/// Validates docs on functions in one Rust source file.
fn validate_file_docs(path: &Path, violations: &mut Vec<Violation>) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read source file {}: {err}", path.display()));
    let lines: Vec<&str> = content.lines().collect();

    for (index, line) in lines.iter().enumerate() {
        let Some(function_name) = parse_function_name(line) else {
            continue;
        };
        if function_name == "main" {
            continue;
        }

        let Some(doc_lines) = collect_function_doc_lines(&lines, index) else {
            continue;
        };

        validate_doc_block(path, index + 1, &doc_lines, violations);
    }
}

/// Parses function name from a line when it begins a Rust function declaration.
fn parse_function_name(line: &str) -> Option<String> {
    let mut trimmed = line.trim_start();

    loop {
        if let Some(rest) = trimmed.strip_prefix("pub ") {
            trimmed = rest;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("async ") {
            trimmed = rest;
            continue;
        }
        if trimmed.starts_with("pub(") {
            let close = trimmed.find(')')?;
            let rest = trimmed.get(close + 1..)?;
            trimmed = rest.trim_start();
            continue;
        }
        break;
    }

    let rest = trimmed.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect();

    if name.is_empty() { None } else { Some(name) }
}

/// Returns contiguous `///` doc lines associated with the function at `function_line`.
fn collect_function_doc_lines<'a>(lines: &'a [&str], function_line: usize) -> Option<Vec<&'a str>> {
    if function_line == 0 {
        return None;
    }

    let mut cursor = function_line;
    let mut seen_doc = false;

    while cursor > 0 {
        let previous = lines[cursor - 1].trim_start();
        if previous.starts_with("///") {
            seen_doc = true;
            cursor -= 1;
            continue;
        }
        if previous.starts_with("#[") || previous.trim().is_empty() {
            cursor -= 1;
            continue;
        }
        break;
    }

    if !seen_doc {
        return None;
    }

    let docs = lines[cursor..function_line]
        .iter()
        .copied()
        .filter(|line| line.trim_start().starts_with("///"))
        .collect::<Vec<_>>();

    Some(docs)
}

/// Validates that one function doc block contains compliant fenced Rust examples.
fn validate_doc_block(path: &Path, line: usize, docs: &[&str], violations: &mut Vec<Violation>) {
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

/// Evaluates one fenced code block against project doctest policy.
fn evaluate_closed_fence(
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
