use anyhow::Result;

use crate::support::root_task_contracts::{cmd_steps, expected_cargo_argv, load_root_module, task};

#[test]
fn repo_root_docs_check_runs_cargo_commands_with_workspace_temp() -> Result<()> {
    let module = load_root_module()?;
    let task = task(&module, "//:docs-check");

    let actual = cmd_steps(task, "docs-check");
    let expected = expected_cargo_argv(&[
        &["test", "--workspace", "--doc"],
        &["test", "-p", "tak", "--test", "doctest_contract"],
        &[
            "test",
            "-p",
            "tak",
            "--test",
            "suite",
            "docs_dump_no_drift_contract",
        ],
    ]);

    assert_eq!(actual, expected, "unexpected //:docs-check steps");
    Ok(())
}
