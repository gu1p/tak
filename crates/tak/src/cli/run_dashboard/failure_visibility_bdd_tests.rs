use ratatui::style::Color;

use super::model::JobActivity;
use super::render_test_support::{frame_at_size, styled_frame};
use super::test_support::state;

#[test]
fn failures_stand_out_in_the_summary_while_other_tasks_keep_running() {
    let mut state = state();
    state.jobs.get_mut("test").unwrap().activity = JobActivity::Failed;
    let rendered = frame_at_size(&state, 100, 24);
    let summary = rendered.split_once("NODES").unwrap().0;
    assert!(summary.contains("1 failed"), "{rendered}");
    let styled = styled_frame(&state, 100, true);
    assert_eq!(styled.style_for("1 failed").fg, Some(Color::Red));
}

#[test]
fn quiet_summary_uses_color_for_status_without_coloring_every_counter() {
    let styled = styled_frame(&state(), 100, true);
    assert_eq!(styled.style_for("RUNNING").fg, Some(Color::Cyan));
    assert_eq!(styled.style_for("0/3 complete").fg, Some(Color::Reset));
}
