mod support;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

use support::run_tak_expect_success;

const REQUIRED_SECTIONS: [&str; 6] = [
    "# Tak Agent Docs",
    "## What Tak Is For",
    "## Core Capabilities",
    "## TASKS.py API Surface",
    "## Example Chooser",
    "## Authoring Workflow",
];

const REQUIRED_TOKENS: [&str; 10] = [
    "module_spec(",
    "task(",
    "cmd(",
    "script(",
    "Local(",
    "Remote(",
    "CurrentState(",
    "small/01_hello_single_task",
    "large/25_remote_direct_build_and_artifact_roundtrip",
    "tak run //:hello",
];

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn docs_dump_succeeds_without_workspace_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let env = BTreeMap::new();

    let output = run_tak_expect_success(temp.path(), &["docs", "dump"], &env)?;

    for section in REQUIRED_SECTIONS {
        assert!(
            output.contains(section),
            "missing section `{section}`:\n{output}"
        );
    }
    for token in REQUIRED_TOKENS {
        assert!(output.contains(token), "missing token `{token}`:\n{output}");
    }

    Ok(())
}

#[test]
fn docs_dump_is_workspace_independent() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let env = BTreeMap::new();

    let temp_output = run_tak_expect_success(temp.path(), &["docs", "dump"], &env)?;
    let repo_output = run_tak_expect_success(repo_root(), &["docs", "dump"], &env)?;

    assert_eq!(
        temp_output, repo_output,
        "docs dump should not depend on cwd"
    );
    Ok(())
}
