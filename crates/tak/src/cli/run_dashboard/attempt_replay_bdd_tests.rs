use tak_proto::local_daemon::v2::RunEventKind;

use super::render_test_support::frame;
use super::test_support::{event, state};

#[test]
fn persisted_reservation_attempt_is_visible_after_a_retry() {
    let mut state = state();
    state
        .apply(&event(1, RunEventKind::Retrying, "build", Some("worker-a")))
        .unwrap();
    let mut reserved = event(2, RunEventKind::Transferring, "build", Some("worker-b"));
    reserved.authored_attempt = Some(2);
    state.apply(&reserved).unwrap();

    let rendered = frame(&state, 118);
    let tasks = rendered
        .split_once("TASKS")
        .and_then(|(_, rest)| rest.split_once("SCHEDULER QUEUE"))
        .map(|(tasks, _)| tasks)
        .expect("tasks panel");
    let row = tasks
        .lines()
        .find(|line| line.contains("//:build"))
        .expect("retried task row");

    assert!(
        row.split_whitespace().any(|field| field == "2"),
        "authored attempt missing after retry:\n{rendered}"
    );
}
