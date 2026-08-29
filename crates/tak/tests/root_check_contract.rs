use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::model::{
    ExecutionPlacementSpec, RemoteRuntimeSpec, SessionReuseSpec, TaskExecutionSpec,
};

use crate::support::root_task_contracts::{load_root_spec, parse};

#[path = "root_check_limiter_contract.rs"]
mod limiter;

#[test]
fn repo_root_check_fuses_the_remote_check_graph() -> Result<()> {
    let spec = load_root_spec()?;
    let task = spec.tasks.get(&parse("//:check")).expect("check task");

    let actual: BTreeSet<_> = task.deps.iter().cloned().collect();
    let expected = BTreeSet::from([
        parse("//:fmt-check"),
        parse("//:line-limits-check"),
        parse("//:src-test-separation-check"),
        parse("//:workflow-contract-check"),
        parse("//:generated-artifact-ignore-check"),
        parse("//:native-dead-code"),
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
                    if remote.session.as_ref().is_some_and(|session|
                        session.display_name == "check-workspace"
                            && matches!(&session.reuse, SessionReuseSpec::Container)
                    )
            ));
            match &placements[0] {
                ExecutionPlacementSpec::Remote(remote) => {
                    assert_eq!(remote.pool, None);
                    assert_eq!(remote.required_tags.as_slice(), ["builder"]);
                    assert_eq!(remote.required_capabilities.as_slice(), ["linux"]);
                    let Some(RemoteRuntimeSpec::Containerized {
                        resource_limits, ..
                    }) = &remote.runtime
                    else {
                        panic!("//:check remote placement should use a container");
                    };
                    assert_eq!(*resource_limits, None);
                }
                _ => unreachable!("remote placement already asserted"),
            }
            assert!(matches!(
                &placements[1],
                ExecutionPlacementSpec::Local(local) if local.session.is_none()
            ));
        }
        other => panic!("//:check should use check workspace execution policy: {other:?}"),
    }
    Ok(())
}
