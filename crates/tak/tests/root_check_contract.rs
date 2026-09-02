use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::v2::{Execution, RemoteSelection, SessionReuse, TaskRuntime};
use tak_loader::{LoadOptions, inspect_authored_root_module};

use crate::support::root_task_contracts::parse;

#[path = "root_check_limiter_contract.rs"]
mod limiter;
#[path = "root_line_limits_contract.rs"]
mod line_limits;
#[path = "root_release_execution_contract.rs"]
mod release_execution;

fn repo_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn repo_root_check_distributes_independent_jobs_through_v2() -> Result<()> {
    let root = inspect_authored_root_module(repo_root(), &LoadOptions::default())?;
    let task = root
        .module
        .tasks
        .iter()
        .find(|task| task.name == "//:check")
        .expect("check task");

    let actual: BTreeSet<_> = task.deps.iter().map(|dep| parse(dep)).collect();
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
    assert!(
        !task.cascade_session,
        "//:check must not cascade one cache policy across every job"
    );
    let session = task.session.as_ref().expect("distributed check session");
    assert_eq!(session.name.as_deref(), Some("check-isolated"));
    assert_eq!(session.reuse, SessionReuse::Workspace);
    assert!(
        session.affinity.is_none(),
        "isolated jobs must have no affinity"
    );
    let Execution::RemoteOnly { remote } = session
        .execution
        .as_deref()
        .expect("distributed placement policy")
    else {
        panic!("//:check must queue on remote builders without a local fallback");
    };
    assert_eq!(remote.selection, RemoteSelection::Balanced);
    assert_eq!(remote.required_tags, ["builder"]);
    assert_eq!(remote.required_capabilities, ["linux"]);
    assert!(matches!(
        remote.runtime,
        Some(TaskRuntime::Container { .. })
    ));
    Ok(())
}
