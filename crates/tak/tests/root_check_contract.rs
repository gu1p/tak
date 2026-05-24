use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::model::{
    ExecutionPlacementSpec, Hold, LimiterDef, RemoteRuntimeSpec, Scope, TaskExecutionSpec,
};

use crate::support::root_task_contracts::{load_root_spec, parse};

const CARGO_CHECK_LOCK: &str = "cargo-check-workspace";

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
            match &placements[0] {
                ExecutionPlacementSpec::Remote(remote) => {
                    let Some(RemoteRuntimeSpec::Containerized {
                        resource_limits: Some(limits),
                        ..
                    }) = &remote.runtime
                    else {
                        panic!("//:check remote placement should use a sized container");
                    };
                    assert_eq!(limits.memory_mb, Some(16_384));
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

#[test]
fn repo_root_cargo_checks_share_one_worktree_lock() -> Result<()> {
    let spec = load_root_spec()?;

    assert!(
        spec.limiters.values().any(|limiter| matches!(
            limiter,
            LimiterDef::Lock { name, scope }
                if name == CARGO_CHECK_LOCK && *scope == Scope::Worktree
        )),
        "missing shared Cargo worktree lock"
    );

    for label in ["//:fmt-check", "//:lint", "//:test", "//:docs-check"] {
        let task = spec.tasks.get(&parse(label)).expect("cargo task");
        assert_eq!(task.needs.len(), 1, "{label} should use one lock need");

        let need = &task.needs[0];
        assert_eq!(need.limiter.name, CARGO_CHECK_LOCK, "{label} lock name");
        assert_eq!(need.limiter.scope, Scope::Worktree, "{label} lock scope");
        assert_eq!(need.slots, 1.0, "{label} lock slots");
        assert!(
            matches!(need.hold, Hold::During),
            "{label} should hold the lock for the full Cargo task"
        );
    }

    Ok(())
}
