use crate::support;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use support::run_tak_expect_success;

const REQUIRED_TASKS_API_SUBSECTIONS: [&str; 4] =
    ["### Types", "### Constants", "### Functions", "### Methods"];
const REQUIRED_TYPED_API_TOKENS: [&str; 10] = [
    "#### `ModuleSpec`",
    "class ModuleSpec(TypedDict):",
    "- `spec_version`: `Literal[1]`",
    "#### `TaskSpec`",
    "#### `MACHINE`",
    "`MACHINE: Literal[\"machine\"]`",
    "-> ModuleSpec: ...",
    "-> TaskSpec: ...",
    "#### `Decision.local`",
    "#### `Decision.remote`",
];

fn run_docs_dump(cwd: &Path) -> Result<String> {
    let env = BTreeMap::new();
    run_tak_expect_success(cwd, &["docs", "dump"], &env)
}

#[test]
fn docs_dump_groups_typed_tasks_api_sections() -> Result<()> {
    let output = run_docs_dump(tempfile::tempdir()?.path())?;

    for section in REQUIRED_TASKS_API_SUBSECTIONS {
        assert!(
            output.contains(section),
            "missing TASKS.py API subsection `{section}`:\n{output}"
        );
    }
    for token in REQUIRED_TYPED_API_TOKENS {
        assert!(
            output.contains(token),
            "missing typed TASKS.py token `{token}`:\n{output}"
        );
    }

    Ok(())
}
