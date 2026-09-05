use base64::{Engine as _, engine::general_purpose::STANDARD};
use tak_proto::local_daemon::v2::RunEventKind;

use super::model::JobActivity;
use super::test_support::{event, state};

#[test]
fn snapshot_exposes_active_node_lanes_and_the_honest_scheduler_queue() {
    let state = state();

    assert_eq!(state.nodes.len(), 2);
    assert_eq!(state.nodes["worker-a"].active_jobs, vec!["//:build"]);
    assert_eq!(state.nodes["worker-b"].active_jobs, vec!["//:test"]);
    assert_eq!(state.scheduler_queue(), vec!["lint"]);
}

#[test]
fn persisted_events_move_a_job_once_and_keep_its_log_tail() {
    let mut state = state();
    state
        .apply(&event(1, RunEventKind::Running, "test", Some("worker-b")))
        .unwrap();
    let mut output = event(2, RunEventKind::Stdout, "test", Some("worker-b"));
    output.chunk_base64 = Some(STANDARD.encode(b"compiled\n"));
    state.apply(&output).unwrap();
    state
        .apply(&event(3, RunEventKind::Succeeded, "test", Some("worker-b")))
        .unwrap();
    state.apply(&output).unwrap();

    assert_eq!(state.jobs["test"].activity, JobActivity::Succeeded);
    assert!(
        !state.nodes["worker-b"]
            .active_jobs
            .contains(&"//:test".into())
    );
    assert_eq!(state.logs.len(), 1, "replayed output must be idempotent");
    assert!(state.logs[0].text.contains("compiled"));
}

#[test]
fn live_logs_name_tasks_instead_of_opaque_daemon_jobs() {
    let mut state = state();
    let mut output = event(1, RunEventKind::Stderr, "build", Some("worker-a"));
    output.task_ids = vec!["//app:compile".into()];
    output.chunk_base64 = Some(STANDARD.encode(b"checking\n"));

    state.apply(&output).unwrap();

    assert_eq!(state.logs[0].job, "//app:compile");
}

#[test]
fn an_older_unseen_event_cannot_rewind_monotonic_replay() {
    let mut state = state();
    state
        .apply(&event(2, RunEventKind::Succeeded, "test", Some("worker-b")))
        .unwrap();
    state
        .apply(&event(1, RunEventKind::Running, "test", Some("worker-b")))
        .unwrap();

    assert_eq!(state.jobs["test"].activity, JobActivity::Succeeded);
    assert!(
        !state.nodes["worker-b"]
            .active_jobs
            .contains(&"//:test".into())
    );
}
