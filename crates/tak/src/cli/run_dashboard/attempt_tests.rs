use tak_proto::local_daemon::v2::RunEventKind;

use super::model::{DashboardJobSeed, DashboardSeed, DashboardState};
use super::render_test_support::frame;
use super::test_support::{event, state};

#[test]
fn attempt_column_distinguishes_unknown_from_daemon_reported_attempts() {
    let rendered = frame(&state(), 118);
    assert_unknown(row(&rendered, "//:lint"), "0");
    assert!(
        row(&rendered, "//:build")
            .split_whitespace()
            .any(|field| field == "1"),
        "{rendered}"
    );
}

#[test]
fn retry_wait_does_not_present_the_completed_attempt_as_current() {
    let mut state = state();
    state
        .apply(&event(1, RunEventKind::Retrying, "build", Some("worker-a")))
        .unwrap();

    let rendered = frame(&state, 118);
    assert_unknown(row(&rendered, "//:build"), "1");
}

#[test]
fn retrying_attach_snapshot_hides_the_completed_attempt() {
    let state = DashboardState::new(DashboardSeed {
        run_id: "run-reattached".into(),
        lifecycle: "running".into(),
        max_parallel_jobs: 1,
        jobs: vec![DashboardJobSeed {
            job_id: "build".into(),
            task_ids: vec!["//:build".into()],
            state: "retrying".into(),
            node_id: None,
            candidate_node_ids: vec!["worker-a".into()],
            queue: None,
            attempt: 3,
            cache: None,
        }],
    });

    let rendered = frame(&state, 80);
    assert_unknown(row(&rendered, "//:build"), "3");
}

#[test]
fn reservation_event_restores_the_daemon_authored_attempt_after_retry() {
    let mut state = state();
    state
        .apply(&event(1, RunEventKind::Retrying, "build", Some("worker-a")))
        .unwrap();
    let mut reserved = event(2, RunEventKind::Transferring, "build", Some("worker-b"));
    reserved.authored_attempt = Some(2);

    state.apply(&reserved).unwrap();

    assert_eq!(state.jobs["build"].attempt, 2);
}

fn row<'a>(rendered: &'a str, task: &str) -> &'a str {
    let tasks = rendered
        .split_once("TASKS")
        .and_then(|(_, rest)| rest.split_once("SCHEDULER QUEUE"))
        .map(|(tasks, _)| tasks)
        .expect("tasks panel");
    tasks
        .lines()
        .find(|line| line.contains(task))
        .expect("task dashboard row")
}

fn assert_unknown(row: &str, false_attempt: &str) {
    assert!(row.contains('—'), "{row}");
    assert!(
        !row.split_whitespace().any(|field| field == false_attempt),
        "{row}"
    );
}
