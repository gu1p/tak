use tak_exec::{PlacementMode, TaskFinishedEvent, TaskStatusEventKind};

use super::super::super::model::{RunState, TaskActivity};
use super::super::{label, row, status};

#[test]
fn cancellation_is_not_relabelled_as_failure_by_final_metadata() {
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
        TaskStatusEventKind::Cancellation,
        None,
        None,
        None,
    ));
    state.apply_finished(TaskFinishedEvent {
        task_run_id: "run-2".into(),
        task_label: label("lint"),
        attempts: 1,
        success: false,
        exit_code: None,
        placement_mode: PlacementMode::Local,
        remote_node_id: None,
    });
    assert_eq!(
        row(&state, &label("lint")).activity,
        TaskActivity::Cancelled
    );
}
