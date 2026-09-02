use anyhow::Result;

use crate::support::root_task_contracts::{cmd_steps, expected_cargo_argv, load_root_module, task};

#[test]
fn repo_root_test_task_runs_workspace_lib_and_integration_tests_with_workspace_temp() -> Result<()>
{
    let module = load_root_module()?;
    let task = task(&module, "//:test");

    let actual = cmd_steps(task, "repo root test task");
    let expected = expected_cargo_argv(&[&[
        "test",
        "--workspace",
        "--lib",
        "--tests",
        "--",
        "--test-threads=1",
    ]]);

    assert_eq!(actual, expected, "unexpected //:test steps");
    Ok(())
}
