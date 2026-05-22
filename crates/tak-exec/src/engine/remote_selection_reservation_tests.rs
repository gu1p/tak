#![cfg(test)]

use tak_core::model::RemoteSelectionSpec;

use super::remote_models::StrictRemoteTransportKind;
use super::remote_selection::{
    RemoteSelectionState, SharedRemoteSelectionState, ordered_remote_targets_for_attempt,
};
use crate::engine::StrictRemoteTarget;

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

#[test]
fn shuffle_reservations_spread_concurrent_first_attempts() {
    let targets = targets(&["a", "b"]);
    let state = SharedRemoteSelectionState::default();

    let first = state.reserve_ordered_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:fmt-check",
        "fmt-run",
        1,
    );
    let second = state.reserve_ordered_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:generated-artifact-ignore-check",
        "generated-run",
        1,
    );

    assert_ne!(first[0].node_id, second[0].node_id);
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
