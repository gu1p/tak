use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};

use super::test_support::{event, state};

#[test]
fn daemon_workspace_cache_events_update_the_task_row() {
    let mut state = state();
    let mut hit = event(1, RunEventKind::Transferring, "lint", Some("worker-a"));
    hit.message = "workspace cache hit".into();
    state.apply(&hit).unwrap();
    assert_eq!(state.jobs["lint"].cache.as_deref(), Some("hit"));

    let mut miss = event(2, RunEventKind::Transferring, "lint", Some("worker-a"));
    miss.message = "workspace cache miss".into();
    state.apply(&miss).unwrap();
    assert_eq!(state.jobs["lint"].cache.as_deref(), Some("miss"));

    let mut unrelated = event(3, RunEventKind::Transferring, "lint", Some("worker-a"));
    unrelated.message = "workspace cache hit elsewhere".into();
    state.apply(&unrelated).unwrap();
    assert_eq!(state.jobs["lint"].cache.as_deref(), Some("miss"));
}

#[test]
fn run_level_cancellation_is_visible_without_inventing_a_job() {
    let mut state = state();
    let mut event = event(1, RunEventKind::Cancelling, "unused", None);
    event.job_id = None;
    event.task_ids.clear();
    state.apply(&event).unwrap();

    assert_eq!(state.lifecycle, "cancelling");
    assert_eq!(state.jobs.len(), 3);
}

#[test]
fn retry_wait_returns_to_the_scheduler_instead_of_claiming_the_old_node() {
    let mut state = state();
    state
        .apply(&event(1, RunEventKind::Retrying, "build", Some("worker-a")))
        .unwrap();

    assert_eq!(state.jobs["build"].node_id, None);
    assert!(state.scheduler_queue().contains(&"build"));
    assert!(state.nodes["worker-a"].active_jobs.is_empty());
}

#[test]
fn scheduler_wait_states_clear_the_previous_attempt_cache_result() {
    let mut state = state();
    assert_eq!(state.jobs["build"].cache.as_deref(), Some("miss"));

    state
        .apply(&event(1, RunEventKind::Retrying, "build", Some("worker-a")))
        .unwrap();
    assert_eq!(state.jobs["build"].cache, None);

    state.jobs.get_mut("build").unwrap().cache = Some("hit".into());
    state
        .apply(&event(2, RunEventKind::Queued, "build", None))
        .unwrap();
    assert_eq!(state.jobs["build"].cache, None);
}

#[test]
fn authoritative_attach_page_state_exposes_retry_capacity_waits() {
    let mut state = state();
    state
        .apply(&event(1, RunEventKind::Running, "build", Some("worker-a")))
        .unwrap();
    state
        .apply(&event(2, RunEventKind::Retrying, "build", Some("worker-a")))
        .unwrap();

    state.sync_lifecycle(RunLifecycleState::Queued);

    assert_eq!(state.lifecycle, "queued");
}
