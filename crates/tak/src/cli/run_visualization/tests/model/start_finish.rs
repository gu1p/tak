use tak_exec::{PlacementMode, TaskFinishedEvent, TaskStartedEvent, TaskStatusEventKind};

use super::super::super::model::{RunState, TaskActivity};
use super::super::{label, row, status};

#[test]
fn local_start_and_finish_become_running_then_passed() {
    let mut state = RunState::new(1);
    state.apply_structured(status(
        "lint",
        TaskStatusEventKind::TaskPlanned,
        None,
        None,
        None,
    ));
    state.apply_started(TaskStartedEvent {
        task_run_id: "run-1".into(),
        task_label: label("lint"),
        placement_mode: PlacementMode::Local,
        remote_node_id: None,
        origin: None,
        runtime: None,
        runtime_source: None,
        command: None,
    });
    assert_eq!(row(&state, &label("lint")).activity, TaskActivity::Running);
    state.apply_finished(TaskFinishedEvent {
        task_run_id: "run-1".into(),
        task_label: label("lint"),
        attempts: 1,
        success: true,
        exit_code: Some(0),
        placement_mode: PlacementMode::Local,
        remote_node_id: None,
    });
    assert_eq!(row(&state, &label("lint")).activity, TaskActivity::Passed);
}
