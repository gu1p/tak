use ratatui::style::Color;

use super::model::{DashboardSeed, DashboardState};
use super::render_test_support::{frame, styled_frame};
use super::test_support::{event, state};
use tak_proto::local_daemon::v2::RunEventKind;

#[test]
fn dashboard_keeps_nodes_tasks_queue_and_live_logs_on_one_operational_surface() {
    let mut state = state();
    let mut output = event(1, RunEventKind::Stdout, "build", Some("worker-a"));
    output.chunk_base64 = Some("YnVpbGQgbG9nCg==".into());
    state.apply(&output).unwrap();

    let rendered = frame(&state, 118);

    for expected in [
        "TAK RUN",
        "run-high-end",
        "NODES",
        "worker-a",
        "worker-b",
        "TASKS",
        "//:build",
        "//:test",
        "SCHEDULER QUEUE",
        "//:lint",
        "candidates: worker-a, worker-b",
        "LIVE LOGS",
        "build log",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected:?}:\n{rendered}"
        );
    }
}

#[test]
fn narrow_dashboard_preserves_reachable_sections_and_explicit_states() {
    let mut state = state();
    state
        .apply(&event(
            1,
            RunEventKind::Cancelling,
            "build",
            Some("worker-a"),
        ))
        .unwrap();

    let rendered = frame(&state, 48);

    for expected in [
        "CANCELLING",
        "NODES",
        "TASKS",
        "SCHEDULER QUEUE",
        "LIVE LOGS",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected:?}:\n{rendered}"
        );
    }
}

#[test]
fn lifecycle_styles_are_semantic_and_can_be_disabled() {
    let state = state();
    let styled = styled_frame(&state, 118, true);
    let plain = styled_frame(&state, 118, false);

    assert_eq!(styled.style_for("RUNNING").fg, Some(Color::Cyan));
    assert_eq!(styled.style_for("ready").fg, Some(Color::Yellow));
    assert_eq!(styled.style_for("running").fg, Some(Color::Cyan));
    assert_eq!(plain.style_for("RUNNING").fg, Some(Color::Reset));
}

#[test]
fn empty_loading_and_error_states_are_explicit() {
    let mut state = DashboardState::new(DashboardSeed {
        run_id: "run-empty".into(),
        lifecycle: "loading".into(),
        max_parallel_jobs: 1,
        jobs: vec![],
    });
    let loading = frame(&state, 72);
    assert!(loading.contains("LOADING") && loading.contains("Waiting for persisted run state"));

    state.lifecycle = "succeeded".into();
    let empty = frame(&state, 72);
    assert!(empty.contains("No executable task steps"), "{empty}");

    state.lifecycle = "failed".into();
    state.error = Some("daemon stream failed".into());
    let failed = frame(&state, 72);
    assert!(failed.contains("FAILED") && failed.contains("daemon stream failed"));
}
