use std::fs;
use std::path::Path;

use super::Violation;
use super::fence::validate_doc_block;
use super::parser::{collect_function_doc_lines, parse_function_name};

/// Validates docs on functions in one Rust source file.
pub(crate) fn validate_file_docs(path: &Path, violations: &mut Vec<Violation>) {
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
