#![cfg(test)]

use tak_core::model::RemoteSelectionSpec;

use self::support::{node_ids, sorted_node_ids, targets};
use super::remote_selection::{RemoteSelectionState, ordered_remote_targets_for_attempt};

#[path = "remote_selection_tests/tests/daemon_fallback.rs"]
mod daemon_fallback;
#[path = "remote_selection_tests/support.rs"]
mod support;

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
