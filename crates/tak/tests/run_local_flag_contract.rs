//! Black-box contracts for local run-mode overrides.

mod support;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use support::{run_tak_expect_failure, run_tak_expect_success, write_tasks};

#[test]
fn run_command_rejects_container_without_mode_selector() -> Result<()> {
    let temp = tempfile::tempdir()?;
    write_tasks(
        temp.path(),
        "SPEC = module_spec(tasks=[task(\"check\", steps=[cmd(\"echo\", \"ok\")])])\nSPEC\n",
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

#[test]
fn run_command_executes_neutral_task_locally_with_local_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let marker = temp.path().join("out/local-run.txt");
    write_tasks(
        temp.path(),
        &format!(
            "SPEC = module_spec(tasks=[task(\"check\", steps=[cmd(\"sh\", \"-c\", \"mkdir -p out && echo local-run > {marker}\")])])\nSPEC\n",
            marker = marker.display()
        ),
    )?;
    let env = BTreeMap::new();
    let stdout = run_tak_expect_success(temp.path(), &["run", "--local", "check"], &env)?;

    assert!(stdout.contains("//:check: ok"), "stdout:\n{stdout}");
    assert!(stdout.contains("placement=local"), "stdout:\n{stdout}");
    assert_eq!(fs::read_to_string(marker)?.trim(), "local-run");
    Ok(())
}
