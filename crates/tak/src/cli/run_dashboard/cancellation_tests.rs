use tak_proto::local_daemon::v2::{RunEvent, RunEventKind};

use super::model::JobActivity;
use super::test_support::{event, state};

#[test]
fn run_level_cancelling_only_leaves_assigned_active_work_cancelling() {
    let mut state = state();
    state
        .apply(&event(
            1,
            RunEventKind::Succeeded,
            "build",
            Some("worker-a"),
        ))
        .unwrap();
    state
        .apply(&run_event(2, RunEventKind::Cancelling))
        .unwrap();

    assert_eq!(state.lifecycle, "cancelling");
    assert_eq!(state.jobs["build"].activity, JobActivity::Succeeded);
    assert_eq!(state.jobs["test"].activity, JobActivity::Cancelling);
    assert_eq!(state.jobs["lint"].activity, JobActivity::Cancelled);
    assert_eq!(
        state.active_jobs(),
        1,
        "unplaced cancellation requests must not consume the run's -j count"
    );
    assert_eq!(state.terminal_jobs(), 2);
    assert!(state.scheduler_queue().is_empty());
}

#[test]
fn run_level_cancelled_marks_every_nonterminal_job_without_rewriting_success() {
    let mut state = state();
    state
        .apply(&event(
            1,
            RunEventKind::Succeeded,
            "build",
            Some("worker-a"),
        ))
        .unwrap();
    state.apply(&run_event(2, RunEventKind::Cancelled)).unwrap();

    assert_eq!(state.lifecycle, "cancelled");
    assert_eq!(state.jobs["build"].activity, JobActivity::Succeeded);
    assert_eq!(state.jobs["test"].activity, JobActivity::Cancelled);
    assert_eq!(state.jobs["lint"].activity, JobActivity::Cancelled);
    assert_eq!(state.active_jobs(), 0);
    assert_eq!(state.terminal_jobs(), 3);
    assert!(state.scheduler_queue().is_empty());
}

fn run_event(seq: u64, kind: RunEventKind) -> RunEvent {
    let mut event = event(seq, kind, "unused", None);
    event.job_id = None;
    event.task_ids.clear();
    event
}
