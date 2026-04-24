use anyhow::Result;

use crate::support::root_task_contracts::{cmd_steps, expected_cargo_argv, load_root_spec, parse};

#[test]
fn repo_root_test_task_runs_full_workspace_cargo_tests_with_workspace_temp() -> Result<()> {
    let spec = load_root_spec()?;
    let task = spec.tasks.get(&parse("//:test")).expect("test task");

    let actual = cmd_steps(task, "repo root test task");
    let expected = expected_cargo_argv(&[&["test", "--workspace"]]);

    assert_eq!(actual, expected, "unexpected //:test steps");
    Ok(())
}
