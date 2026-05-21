#![cfg(test)]

use tak_core::model::RemoteSelectionSpec;

use super::remote_models::StrictRemoteTransportKind;
use super::remote_selection::{RemoteSelectionState, ordered_remote_targets_for_attempt};
use crate::engine::StrictRemoteTarget;

#[test]
fn sequential_selection_preserves_inventory_order() {
    let targets = targets(&["a", "b", "c"]);
    let state = RemoteSelectionState::default();

    let ordered = ordered_remote_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Sequential,
        "//:check",
        "run-1",
        1,
        &state,
    );

    assert_eq!(node_ids(&ordered), ["a", "b", "c"]);
}

#[test]
fn shuffle_selection_is_deterministic_for_task_run_and_attempt() {
    let targets = targets(&["a", "b", "c", "d", "e"]);
    let state = RemoteSelectionState::default();

    let first = ordered_remote_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:check",
        "run-1",
        1,
        &state,
    );
    let repeated = ordered_remote_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:check",
        "run-1",
        1,
        &state,
    );
    let next_attempt = ordered_remote_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:check",
        "run-1",
        2,
        &state,
    );

    assert_eq!(node_ids(&first), node_ids(&repeated));
    assert_ne!(node_ids(&first), node_ids(&next_attempt));
    assert_eq!(sorted_node_ids(&first), ["a", "b", "c", "d", "e"]);
}

#[test]
fn shuffle_selection_balances_assignments_across_equal_targets() {
    let targets = targets(&["a", "b"]);
    let mut state = RemoteSelectionState::default();

    for index in 0..6 {
        let ordered = ordered_remote_targets_for_attempt(
            &targets,
            RemoteSelectionSpec::Shuffle,
            "//:check",
            &format!("run-{index}"),
            1,
            &state,
        );
        state.record_assignment(&ordered[0].node_id);
    }

    assert_eq!(state.assignment_count("a"), 3);
    assert_eq!(state.assignment_count("b"), 3);
}

fn targets(ids: &[&str]) -> Vec<StrictRemoteTarget> {
    ids.iter()
        .map(|id| StrictRemoteTarget {
            node_id: (*id).to_string(),
            endpoint: "http://127.0.0.1:1".to_string(),
            transport_kind: StrictRemoteTransportKind::Direct,
            bearer_token: "secret".to_string(),
            runtime: None,
        })
        .collect()
}

fn node_ids(targets: &[StrictRemoteTarget]) -> Vec<&str> {
    targets
        .iter()
        .map(|target| target.node_id.as_str())
        .collect()
}

fn sorted_node_ids(targets: &[StrictRemoteTarget]) -> Vec<&str> {
    let mut ids = node_ids(targets);
    ids.sort_unstable();
    ids
}
