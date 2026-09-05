use tak_proto::local_daemon::v2::RunEventKind;

use super::render_test_support::frame;
use super::terminal::final_summary;
use super::test_support::{event, state};

#[test]
fn failed_job_diagnostic_survives_the_tui_and_final_summary() {
    let mut state = state();
    let mut failure = event(1, RunEventKind::Failed, "job-0", Some("worker-a"));
    failure.task_ids = vec!["//:build".into()];
    failure.message = "worker process exited 17 after losing its output layer".into();
    state.apply(&failure).unwrap();

    let rendered = frame(&state, 118);
    let summary = final_summary(&state);

    assert!(
        rendered.contains(&failure.message),
        "failure diagnostic disappeared from dashboard:\n{rendered}"
    );
    assert!(rendered.contains("//:build@worker-a"), "{rendered}");
    assert!(!rendered.contains("job-0@worker-a"), "{rendered}");
    assert!(summary.contains(&failure.message), "{summary}");
    assert!(summary.contains("//:build@worker-a"), "{summary}");
    assert!(!summary.contains("job-0@worker-a"), "{summary}");
}

#[test]
fn run_failure_diagnostic_is_visible_even_when_the_run_has_jobs() {
    let mut state = state();
    let mut failure = event(1, RunEventKind::Failed, "unused", None);
    failure.job_id = None;
    failure.task_ids.clear();
    failure.message = "output conflict at dist/result.json".into();
    state.apply(&failure).unwrap();

    let rendered = frame(&state, 118);

    assert!(rendered.contains(&failure.message), "{rendered}");
    assert!(final_summary(&state).contains(&failure.message));
}
