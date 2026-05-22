#![cfg(test)]

use tak_core::model::RemoteSelectionSpec;

use super::release_aggregate_shuffle_reservation;
use crate::engine::remote_selection::SharedRemoteSelectionState;

#[path = "session_cascade_selection_tests_support.rs"]
mod support;

use support::{
    aggregate_task, container_session, placement_with_session, run_id_that_prefers, targets,
};

#[test]
fn aggregate_shuffle_reservation_is_retained_for_container_session_placement() {
    let targets = targets();
    let state = SharedRemoteSelectionState::default();
    let first = state.reserve_ordered_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:root-a",
        "run-a",
        1,
    );
    let selected = first[0].node_id.clone();
    let task = aggregate_task();
    let placement = placement_with_session(&selected, Some(container_session()));
    let run_id = run_id_that_prefers(&targets, &selected);

    release_aggregate_shuffle_reservation(&task, &placement, &state);

    let second = state.reserve_ordered_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:root-b",
        &run_id,
        1,
    );
    assert_ne!(second[0].node_id, selected);
}

#[test]
fn aggregate_shuffle_reservation_is_released_for_non_container_aggregate() {
    let targets = targets();
    let state = SharedRemoteSelectionState::default();
    let first = state.reserve_ordered_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:root-a",
        "run-a",
        1,
    );
    let selected = first[0].node_id.clone();
    let task = aggregate_task();
    let placement = placement_with_session(&selected, None);
    let run_id = run_id_that_prefers(&targets, &selected);

    release_aggregate_shuffle_reservation(&task, &placement, &state);

    let second = state.reserve_ordered_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:root-b",
        &run_id,
        1,
    );
    assert_eq!(second[0].node_id, selected);
}
