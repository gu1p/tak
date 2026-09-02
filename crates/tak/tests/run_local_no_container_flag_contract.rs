//! Black-box contracts for `tak run --local-no-container`.

use crate::support::{run_tak_expect_failure, write_tasks};

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;

#[test]
fn run_command_local_no_container_rejects_remote_and_container_flags() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        "SPEC = module_spec(spec_version=2, tasks=[task(\"check\")])\nSPEC\n",
    )?;
    assert_rejected(
        temp.path(),
        &["run", "--local-no-container", "--remote", "check"],
        "--local-no-container and --remote are mutually exclusive",
    )?;
    assert_rejected(
        temp.path(),
        &["run", "--local-no-container", "--container", "check"],
        "--local-no-container cannot be combined with container flags",
    )?;
    assert_rejected(
        temp.path(),
        &[
            "run",
            "--local-no-container",
            "--container-image",
            "alpine:3.20",
            "check",
        ],
        "--local-no-container cannot be combined with container flags",
    )
}

fn assert_rejected(workspace: &Path, args: &[&str], expected: &str) -> Result<()> {
    let (_stdout, stderr) = run_tak_expect_failure(workspace, args, &BTreeMap::new())?;
    assert!(stderr.contains(expected), "stderr:\n{stderr}");
    Ok(())
}
