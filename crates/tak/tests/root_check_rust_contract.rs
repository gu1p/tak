use std::collections::BTreeSet;

use anyhow::Result;

use crate::support::root_task_contracts::{load_root_module, task};

#[test]
fn repo_root_check_rust_aggregates_lint_test_and_docs() -> Result<()> {
    let module = load_root_module()?;
    let task = task(&module, "//:check-rust");

    let actual: BTreeSet<_> = task.deps.iter().cloned().collect();
    let expected = BTreeSet::from([
        "//:lint".to_string(),
        "//:test".to_string(),
        "//:docs-check".to_string(),
    ]);

    assert_eq!(actual, expected, "unexpected //:check-rust deps");
    assert!(
        task.steps.is_empty(),
        "//:check-rust should be an aggregate task"
    );
    Ok(())
}
