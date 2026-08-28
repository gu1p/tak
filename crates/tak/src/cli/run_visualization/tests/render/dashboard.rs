use tak_exec::TaskStatusEventKind;

use super::super::super::model::RunState;
use super::super::super::render::render_plain;
use super::super::{backdate, label, status};

#[test]
fn dashboard_has_operational_headers_node_strip_and_fused_count() {
    let mut state = RunState::new(2);
    let mut planned = status("check", TaskStatusEventKind::TaskPlanned, None, None, None);
    planned.execution_unit_members.push(label("lint"));
    state.apply_structured(planned);
    state.apply_structured(status(
        "later",
        TaskStatusEventKind::TaskPlanned,
        None,
        None,
        None,
    ));
    state.apply_structured(status(
        "check",
        TaskStatusEventKind::QueueAdmission,
        Some("worker"),
        Some(2),
        Some("node-a"),
    ));

    let rendered = render_plain(&state, 100);
    for expected in [
        "tak run",
        "1/2 jobs",
        "TASK",
        "PLACEMENT",
        "ACTIVITY",
        "node-a ×1",
        "<1s",
    ] {
        assert!(rendered.contains(expected), "{rendered}");
    }
    assert!(rendered.contains("//:check (+1 task)"), "{rendered}");
    assert!(
        rendered.contains("queued · worker #2 (1 ahead)"),
        "{rendered}"
    );
}

#[test]
fn large_runs_keep_active_work_visible_and_collapse_old_successes() {
    let mut state = RunState::new(4);
    for index in 0..14 {
        let name = format!("task-{index}");
        state.apply_structured(status(
            &name,
            TaskStatusEventKind::TaskPlanned,
            None,
            None,
            None,
        ));
        if index < 12 {
            state.apply_structured(status(
                &name,
                TaskStatusEventKind::Completion,
                None,
                None,
                None,
            ));
        }
    }
    let rendered = render_plain(&state, 100);
    assert!(rendered.contains("//:task-12"), "{rendered}");
    assert!(rendered.contains("//:task-13"), "{rendered}");
    assert!(rendered.contains("completed tasks hidden"), "{rendered}");
    assert!(!rendered.contains("//:task-0 "), "{rendered}");
}

#[test]
fn elapsed_time_is_recomputed_when_a_live_frame_refreshes() {
    let mut state = RunState::new(1);
    state.apply_structured(status(
        "lint",
        TaskStatusEventKind::TaskPlanned,
        None,
        None,
        None,
    ));
    backdate(
        &mut state,
        &label("lint"),
        std::time::Duration::from_secs(65),
    );
    let rendered = render_plain(&state, 100);
    assert!(rendered.contains("1m 05s"), "{rendered}");
}
