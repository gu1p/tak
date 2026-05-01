use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::model::{ExecutionPlacementSpec, TaskExecutionSpec};

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
    match &task.execution {
        TaskExecutionSpec::ByExecutionPolicy { placements, .. } => {
            assert!(
                task.cascade_execution,
                "//:check should cascade its selected execution"
            );
            assert_eq!(placements.len(), 2);
            assert!(matches!(
                &placements[0],
                ExecutionPlacementSpec::Remote(remote)
                    if remote.session.as_ref().is_some_and(|session| session.display_name == "check-workspace")
            ));
            assert!(matches!(
                &placements[1],
                ExecutionPlacementSpec::Local(local) if local.session.is_none()
            ));
        }
        other => panic!("//:check should use check workspace execution policy: {other:?}"),
    }
    Ok(())
}
