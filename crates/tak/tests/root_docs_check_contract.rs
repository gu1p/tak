use anyhow::Result;

use crate::support::root_task_contracts::{cmd_steps, expected_cargo_argv, load_root_spec, parse};

#[test]
fn repo_root_docs_check_runs_cargo_commands_with_workspace_temp() -> Result<()> {
    let spec = load_root_spec()?;
    let task = spec
        .tasks
        .get(&parse("//:docs-check"))
        .expect("docs-check task");

    let actual = cmd_steps(task, "docs-check");
    let expected = expected_cargo_argv(&[
        &["test", "--workspace", "--doc"],
        &["test", "-p", "tak", "--test", "doctest_contract"],
    ]);

    assert_eq!(actual, expected, "unexpected //:docs-check steps");
    Ok(())
}
