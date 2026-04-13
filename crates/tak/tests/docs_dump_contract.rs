mod support;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
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
const REQUIRED_SOURCE_TOKENS: [&str; 8] = [
    "#### Source Files",
    "##### `apps/web/TASKS.py`",
    "##### `services/api/TASKS.py`",
    "# Example: small/01_hello_single_task",
    "doc=\"Writes a hello output file.\"",
    "# File: apps/web/TASKS.py",
    "deps=[\"//apps/api:build\", \"//libs/common:lint\"]",
    "execution=RemoteOnly(REMOTE)",
];

fn repo_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

fn run_docs_dump(cwd: &Path) -> Result<String> {
    let env = BTreeMap::new();
    run_tak_expect_success(cwd, &["docs", "dump"], &env)
}
#[test]
fn docs_dump_succeeds_without_workspace_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let output = run_docs_dump(temp.path())?;

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
fn docs_dump_embeds_recommended_example_sources() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let output = run_docs_dump(temp.path())?;

    for token in REQUIRED_SOURCE_TOKENS {
        assert!(
            output.contains(token),
            "missing source token `{token}`:\n{output}"
        );
    }

    Ok(())
}
#[test]
fn docs_dump_is_workspace_independent() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let temp_output = run_docs_dump(temp.path())?;
    let repo_output = run_docs_dump(repo_root())?;

    assert_eq!(
        temp_output, repo_output,
        "docs dump should not depend on cwd"
    );
    Ok(())
}
