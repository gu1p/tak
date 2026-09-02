//! Black-box contracts for local run-mode overrides.

use crate::support;

use anyhow::Result;
use std::collections::BTreeMap;

use support::{run_tak_expect_failure, write_tasks};

#[test]
fn run_command_rejects_container_without_mode_selector() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        "SPEC = module_spec(spec_version=2, tasks=[task(\"check\", steps=[cmd(\"echo\", \"ok\")])])\nSPEC\n",
    )?;
    let env = BTreeMap::new();
    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["run", "--container", "check"], &env)?;

    assert!(
        stderr.contains("--container requires exactly one of --local or --remote"),
        "stderr:\n{stderr}"
    );
    Ok(())
}
