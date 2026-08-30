use takd::{AttemptCompletion, RunStore, SchedulerNode};

use crate::support::v2_run::{
    constraints::{project_lock, project_queue},
    scheduler::independent_jobs,
};
use tak_core::v2::HoldMode;

#[test]
fn active_scoped_definition_conflicts_show_both_definitions() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = project_queue(independent_jobs("definition-a", 1), 1);
    let second = project_queue(independent_jobs("definition-b", 1), 2);
    store.submit(&first, "alice").unwrap();

    let error = store.submit(&second, "bob").unwrap_err().to_string();
    assert!(error.contains("conflicting queue definition"));
    assert!(error.contains(&serde_json::to_string(&first.run.queue_definitions[0]).unwrap()));
    assert!(error.contains(&serde_json::to_string(&second.run.queue_definitions[0]).unwrap()));
}

#[test]
fn active_limiter_conflicts_show_both_definitions() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = project_lock(independent_jobs("limiter-a", 1), HoldMode::AtStart);
    let second = project_lock(independent_jobs("limiter-b", 1), HoldMode::During);
    store.submit(&first, "alice").unwrap();

    let error = store.submit(&second, "bob").unwrap_err().to_string();
    assert!(error.contains("conflicting limiter definition"));
    assert!(error.contains(&serde_json::to_string(&first.run.limiter_definitions[0]).unwrap()));
    assert!(error.contains(&serde_json::to_string(&second.run.limiter_definitions[0]).unwrap()));
}

#[test]
fn a_changed_definition_is_accepted_after_the_previous_run_is_terminal() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let first = project_lock(independent_jobs("terminal-a", 1), HoldMode::AtStart);
    crate::support::v2_run::scheduler::commit(&store, &first, "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1)])
        .unwrap()
        .unwrap();
    store
        .complete_attempt(
            &command,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
    let changed = project_lock(independent_jobs("terminal-b", 1), HoldMode::During);
    assert!(store.submit(&changed, "bob").is_ok());
}
