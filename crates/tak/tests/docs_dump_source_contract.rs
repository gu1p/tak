use crate::support;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use support::run_tak_expect_success;

const REQUIRED_CLI_DOC_TOKENS: [&str; 4] = [
    "List every fully-qualified task label available from the current workspace",
    "Execute one or more task labels plus their dependencies",
    "Continue scheduling independent tasks after a task failure",
    "Refresh the node snapshot every N milliseconds while watching",
];
const REQUIRED_DSL_DOC_TOKENS: [&str; 4] = [
    "Declare the module boundary that Tak loads from one TASKS.py file.",
    "Declare one task, including its steps, dependencies, execution policy, and outputs.",
    "Describe a remote execution target by pool, capability filters, transport, and runtime.",
    "Capture the current workspace contents as an execution input snapshot.",
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
const REQUIRED_TYPED_STUB_DOC_TOKENS: [&str; 4] = [
    "Top-level TASKS.py module payload returned by `module_spec(...)`.",
    "Task dictionary returned by `task(...)` after dependency normalization.",
    "Machine-wide coordination scope.",
    "No public methods are currently exposed by the shipped TASKS.py DSL.",
];

fn run_docs_dump(cwd: &Path) -> Result<String> {
    let env = BTreeMap::new();
    run_tak_expect_success(cwd, &["docs", "dump"], &env)
}

#[test]
fn docs_dump_uses_cli_source_docs() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    for token in REQUIRED_CLI_DOC_TOKENS {
        assert!(
            output.contains(token),
            "missing CLI doc `{token}`:\n{output}"
        );
    }
    Ok(())
}

#[test]
fn docs_dump_uses_dsl_docstrings() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    for token in REQUIRED_DSL_DOC_TOKENS {
        assert!(
            output.contains(token),
            "missing DSL doc `{token}`:\n{output}"
        );
    }
    Ok(())
}

#[test]
fn docs_dump_uses_stub_docs_for_types_and_constants() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    for token in REQUIRED_TYPED_STUB_DOC_TOKENS {
        assert!(
            output.contains(token),
            "missing typed stub doc `{token}`:\n{output}"
        );
    }
    Ok(())
}

#[test]
fn docs_dump_uses_example_comments_and_task_docs() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;
    for token in REQUIRED_EXAMPLE_DOC_TOKENS {
        assert!(
            output.contains(token),
            "missing example source doc `{token}`:\n{output}"
        );
    }
    Ok(())
}
