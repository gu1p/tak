use ratatui::style::Color;
use tak_exec::TaskStatusEventKind;

use super::super::super::model::{RunState, TaskActivity};
use super::super::super::render::{activity_color, render_frame};
use super::super::status;

#[test]
fn semantic_colors_keep_terminal_states_distinct() {
    assert_eq!(activity_color(TaskActivity::Passed), Color::Green);
    assert_eq!(activity_color(TaskActivity::Failed), Color::Red);
    assert_eq!(activity_color(TaskActivity::Queued), Color::Yellow);
    assert_eq!(activity_color(TaskActivity::Uploading), Color::Magenta);
}

#[test]
fn terminal_frame_uses_color_but_redirected_frame_never_uses_ansi() {
    let mut state = RunState::new(1);
    state.apply_structured(status(
        "lint",
        TaskStatusEventKind::TaskPlanned,
        None,
        None,
        None,
    ));
    state.apply_structured(status(
        "lint",
        TaskStatusEventKind::QueueAdmission,
        Some("worker"),
        Some(1),
        Some("node-a"),
    ));
    let colored = render_frame(&state, 100, true);
    let redirected = render_frame(&state, 100, false);
    assert!(colored.contains("\u{1b}["), "{colored:?}");
    assert!(!redirected.contains("\u{1b}["), "{redirected:?}");
}
