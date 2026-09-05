use super::model::{DashboardJobSeed, DashboardSeed, DashboardState, LogLine};
use super::navigation::{DashboardNavigation, NavigationAction};
use super::render_test_support::frame_at_size_with_navigation;

#[test]
fn keyboard_navigation_reaches_every_operational_row_in_a_large_run() {
    let mut state = large_state();
    state.logs = (0..30)
        .map(|index| LogLine {
            job: format!("//:task-{index:02}"),
            node: format!("worker-{index:02}"),
            text: format!("log-{index:02}"),
        })
        .collect();
    let mut navigation = DashboardNavigation::default();

    navigation.apply(NavigationAction::End);
    assert_visible(&state, &navigation, "//:task-29");

    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::End);
    assert_visible(&state, &navigation, "queue: builds-29");

    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::Home);
    assert_visible(&state, &navigation, "log-00");
    navigation.apply(NavigationAction::End);
    assert_visible(&state, &navigation, "log-29");

    navigation.apply(NavigationAction::NextPanel);
    navigation.apply(NavigationAction::End);
    assert_visible(&state, &navigation, "worker-29");
}

#[test]
fn dashboard_exposes_focus_and_complete_keyboard_help() {
    let state = large_state();
    let navigation = DashboardNavigation::default();
    let rendered = frame_at_size_with_navigation(&state, &navigation, 100, 24);

    for expected in [
        "▶ TASKS",
        "Tab panel",
        "↑↓ scroll",
        "PgUp/PgDn",
        "Home/End",
        "Ctrl-C cancel",
        "30 queued",
        "0/30 complete",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected:?}:\n{rendered}"
        );
    }
}

fn assert_visible(state: &DashboardState, navigation: &DashboardNavigation, expected: &str) {
    let rendered = frame_at_size_with_navigation(state, navigation, 100, 24);
    assert!(
        rendered.contains(expected),
        "missing {expected:?}:\n{rendered}"
    );
}

fn large_state() -> DashboardState {
    let jobs = (0..30)
        .map(|index| DashboardJobSeed {
            job_id: format!("job-{index:02}"),
            task_ids: vec![format!("//:task-{index:02}")],
            state: "ready".into(),
            node_id: None,
            candidate_node_ids: vec![format!("worker-{index:02}")],
            queue: Some(format!("builds-{index:02}")),
            attempt: 0,
            cache: None,
        })
        .collect();
    DashboardState::new(DashboardSeed {
        run_id: "run-large-navigation".into(),
        lifecycle: "queued".into(),
        max_parallel_jobs: 4,
        jobs,
    })
}
