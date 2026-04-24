use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::model::TaskExecutionSpec;

use crate::support::root_task_contracts::{load_root_spec, parse};

#[test]
fn repo_root_check_runs_light_checks_then_shared_rust_lane() -> Result<()> {
    let spec = load_root_spec()?;
    let task = spec.tasks.get(&parse("//:check")).expect("check task");

    let actual: BTreeSet<_> = task.deps.iter().cloned().collect();
    let expected = BTreeSet::from([
        parse("//:fmt-check"),
        parse("//:line-limits-check"),
        parse("//:src-test-separation-check"),
        parse("//:workflow-contract-check"),
        parse("//:generated-artifact-ignore-check"),
        parse("//:check-rust"),
    ]);

    assert_eq!(actual, expected, "unexpected //:check deps");
    assert!(
        task.steps.is_empty(),
        "//:check should be an aggregate task"
    );
    assert!(spec.sessions.contains_key("check-workspace"));
    match &task.execution {
        TaskExecutionSpec::UseSession { name, cascade } => {
            assert_eq!(name, "check-workspace");
            assert!(*cascade, "//:check should cascade its shared session");
        }
        other => panic!("//:check should use check-workspace session: {other:?}"),
    }
    Ok(())
}
