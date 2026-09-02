use crate::support;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use support::run_tak_expect_success;

const REQUIRED_CLI_DOC_TOKENS: [&str; 6] = [
    "List every task from the current workspace with its label, dependencies, and description",
    "Execute one or more task labels plus their dependencies",
    "Execute one Makefile goal through Tak's runtime selection",
    "Continue scheduling independent tasks after a task failure",
    "Force local host execution without a container",
    "Refresh the node snapshot every N milliseconds while watching",
];
const REQUIRED_ROOT_CLI_DOC_TOKENS: [&str; 7] = [
    "### `tak`",
    "`tak --node`",
    "`tak --arch`",
    "`tak --pool`",
    "`tak --tag`",
    "`tak --capability`",
    "`tak --transport`",
];
const REQUIRED_DSL_DOC_TOKENS: [&str; 4] = [
    "Declare a version 2 module boundary loaded from one TASKS.py file.",
    "Declare one version 2 task and its daemon execution contract.",
    "Force daemon-owned scheduling onto matching remote workers.",
    "Capture current workspace contents as an execution input snapshot.",
];
const REQUIRED_EXAMPLE_DOC_TOKENS: [&str; 6] = [
    "- Scenario: hello single task",
    "- Task docs:",
    "- `hello`: Writes a hello output file.",
    "- Scenario: remote direct build and artifact roundtrip",
    "- `build_remote`: Build the service remotely and return the declared artifact directory.",
    "- `release`: Join the remote artifact and the local verification log into one release \
summary.",
];
const REQUIRED_TYPED_STUB_DOC_TOKENS: [&str; 5] = [
    "Top-level version 2 TASKS.py payload returned by `module_spec(...)`.",
    "Version 2 task dictionary returned by `task(...)`.",
    "Machine-wide coordination scope.",
    "Return an explicit local placement decision from a custom policy.",
    "Share one session workspace with bounded task concurrency.",
];

fn run_docs_dump(cwd: &Path) -> Result<String> {
    let env = BTreeMap::new();
    run_tak_expect_success(cwd, &["docs", "dump"], &env)
}

#[test]
fn docs_dump_uses_cli_source_docs() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    assert_contains_all(&output, &REQUIRED_CLI_DOC_TOKENS, "CLI doc");
    assert_contains_all(&output, &REQUIRED_ROOT_CLI_DOC_TOKENS, "root CLI doc");
    Ok(())
}

#[test]
fn docs_dump_uses_dsl_docstrings() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    assert_contains_all(&output, &REQUIRED_DSL_DOC_TOKENS, "DSL doc");
    Ok(())
}

#[test]
fn docs_dump_uses_stub_docs_for_types_and_constants() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    assert_contains_all(&output, &REQUIRED_TYPED_STUB_DOC_TOKENS, "typed stub doc");
    Ok(())
}

#[test]
fn docs_dump_uses_example_comments_and_task_docs() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    assert_contains_all(&output, &REQUIRED_EXAMPLE_DOC_TOKENS, "example source doc");
    Ok(())
}

fn assert_contains_all(output: &str, tokens: &[&str], label: &str) {
    for token in tokens {
        assert!(
            output.contains(token),
            "missing {label} `{token}`:\n{output}"
        );
    }
}
