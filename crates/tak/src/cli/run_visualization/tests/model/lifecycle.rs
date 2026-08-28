use tak_exec::TaskStatusEventKind;

use super::super::super::model::{RunState, TaskActivity};
use super::super::{label, row, status};

#[test]
fn lifecycle_tracks_fused_members_queues_and_concrete_node_replacement() {
    let mut state = RunState::new(2);
    let mut planned = status("check", TaskStatusEventKind::TaskPlanned, None, None, None);
    planned.execution_unit_members = vec![label("lint"), label("check")];
    state.apply_structured(planned);
    state.apply_structured(status(
        "check",
        TaskStatusEventKind::QueueAdmission,
        Some("scheduler"),
        Some(2),
        None,
    ));
    assert_eq!(row(&state, &label("check")).member_count, 2);
    assert_eq!(row(&state, &label("check")).activity, TaskActivity::Waiting);

    state.apply_structured(status(
        "check",
        TaskStatusEventKind::Dispatch,
        Some("scheduler"),
        None,
        None,
    ));
    let dispatched = row(&state, &label("check"));
    assert_eq!(dispatched.activity, TaskActivity::Placing);
    assert_eq!(dispatched.queue_id, None);

    state.apply_structured(status(
        "check",
        TaskStatusEventKind::QueueAdmission,
        Some("worker"),
        Some(2),
        Some("node-a"),
    ));
    let queued = row(&state, &label("check"));
    assert_eq!(queued.activity, TaskActivity::Queued);
    assert_eq!(queued.node.as_deref(), Some("node-a"));
    assert_eq!(queued.queue_position, Some(2));

    state.apply_structured(status(
        "check",
        TaskStatusEventKind::RemoteCapacityDiscovery,
        None,
        None,
        None,
    ));
    assert_eq!(row(&state, &label("check")).node.as_deref(), Some("node-a"));

    let mut failover = status(
        "check",
        TaskStatusEventKind::UploadProgress,
        None,
        None,
        Some("node-b"),
    );
    failover.transport = Some("tor".into());
    state.apply_structured(failover);
    let failed_over = row(&state, &label("check"));
    assert_eq!(failed_over.node.as_deref(), Some("node-b"));
    assert_eq!(failed_over.transport.as_deref(), Some("tor"));
}
