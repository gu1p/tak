use std::collections::BTreeSet;

use anyhow::Result;

use crate::support::root_task_contracts::{load_root_spec, parse};

#[test]
fn repo_root_check_rust_aggregates_lint_test_and_docs() -> Result<()> {
    let spec = load_root_spec()?;
    let task = spec
        .tasks
        .get(&parse("//:check-rust"))
        .expect("check-rust task");

    let actual: BTreeSet<_> = task.deps.iter().cloned().collect();
    let expected = BTreeSet::from([parse("//:lint"), parse("//:test"), parse("//:docs-check")]);

    assert_eq!(actual, expected, "unexpected //:check-rust deps");
    assert!(
        task.steps.is_empty(),
        "//:check-rust should be an aggregate task"
    );
    Ok(())
}
