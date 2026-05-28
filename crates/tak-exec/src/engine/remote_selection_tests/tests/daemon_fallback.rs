use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind};

use crate::engine::StrictRemoteTarget;

use super::super::remote_selection::{RemoteSelectionState, ordered_remote_targets_for_attempt};
use super::support::targets;

#[test]
fn shuffle_selection_keeps_daemon_tor_fallback_after_concrete_targets() {
    let mut targets = targets(&["a", "b", "c"]);
    targets.push(StrictRemoteTarget::daemon_tor_placement(&RemoteSpec {
        pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        transport_kind: RemoteTransportKind::Any,
        runtime: None,
        selection: RemoteSelectionSpec::Shuffle,
        session: None,
    }));
    let state = RemoteSelectionState::default();

    let ordered = ordered_remote_targets_for_attempt(
        &targets,
        RemoteSelectionSpec::Shuffle,
        "//:check",
        "run-1",
        1,
        &state,
    );

    assert_eq!(
        ordered.last().map(|target| target.node_id.as_str()),
        Some("__takd_daemon_tor__")
    );
}
