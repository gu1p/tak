use super::model::{DashboardSeed, DashboardState, LogLine};
use super::navigation::{DashboardNavigation, NavigationAction};
use super::render_test_support::{frame_at_size, frame_at_size_with_navigation};
use super::test_support::state;

#[test]
fn short_terminal_keeps_focused_content_and_navigation_usable() {
    let mut state = state();
    state.logs.push(LogLine {
        job: "build".into(),
        node: "worker-a".into(),
        text: "visible output".into(),
    });
    let mut navigation = DashboardNavigation::default();
    let tasks = frame_at_size_with_navigation(&state, &navigation, 60, 12);
    for expected in ["TASKS", "//:build", "Tab panel", "Ctrl-C cancel"] {
        assert!(tasks.contains(expected), "missing {expected}:\n{tasks}");
    }
    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::NextPanel);
    let logs = frame_at_size_with_navigation(&state, &navigation, 60, 12);
    assert!(logs.contains("visible output"), "{logs}");
}

#[test]
fn loading_and_error_screens_keep_help_and_wrap_diagnostics() {
    let mut state = DashboardState::new(DashboardSeed {
        run_id: "empty".into(),
        lifecycle: "loading".into(),
        max_parallel_jobs: 1,
        jobs: vec![],
    });
    let loading = frame_at_size(&state, 48, 12);
    assert!(loading.contains("Ctrl-C cancel"), "{loading}");
    state.lifecycle = "failed".into();
    state.error = Some("Could not read the run from the daemon: connection refused".into());
    let failed = frame_at_size(&state, 48, 12);
    assert!(failed.contains("connection refused"), "{failed}");
}

#[test]
fn narrow_cancellation_notice_is_not_cut_off() {
    let mut state = state();
    state.note_cancellation_persisted();
    let rendered = frame_at_size(&state, 48, 30);
    assert!(rendered.contains("stop active work"), "{rendered}");
}

#[test]
fn overflowing_panels_show_the_visible_range_when_scrolling() {
    let mut state = state();
    state.logs = (0..30)
        .map(|index| LogLine {
            job: "build".into(),
            node: "local".into(),
            text: format!("output-{index:02}"),
        })
        .collect();
    let mut navigation = DashboardNavigation::default();
    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::Home);
    let first = frame_at_size_with_navigation(&state, &navigation, 80, 24);
    assert!(
        first.contains("of 30") && first.contains("output-00"),
        "{first}"
    );
    navigation.apply(NavigationAction::End);
    let last = frame_at_size_with_navigation(&state, &navigation, 80, 24);
    assert!(
        last.contains("30 of 30") && last.contains("output-29"),
        "{last}"
    );
}
