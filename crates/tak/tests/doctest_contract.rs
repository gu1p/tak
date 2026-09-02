//! Contract tests that enforce doctest coverage and policy for source function docs.

#[path = "doctest_contract/docs.rs"]
mod docs;
#[path = "doctest_contract/fence.rs"]
mod fence;
#[path = "doctest_contract/fence_policy.rs"]
mod fence_policy;
#[path = "doctest_contract/parser.rs"]
mod parser;
#[path = "doctest_contract/source_files.rs"]
mod source_files;

use std::path::PathBuf;

use docs::validate_file_docs;
use source_files::{collect_rust_source_files, repo_root};

/// One docs policy violation discovered by the contract scanner.
#[derive(Debug)]
pub(crate) struct Violation {
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    pub(crate) message: String,
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
