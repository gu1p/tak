use crate::engine::StrictRemoteTarget;

use super::super::remote_models::StrictRemoteTransportKind;

pub(super) fn targets(ids: &[&str]) -> Vec<StrictRemoteTarget> {
    ids.iter()
        .map(|id| StrictRemoteTarget {
            node_id: (*id).to_string(),
            endpoint: "http://127.0.0.1:1".to_string(),
            transport_kind: StrictRemoteTransportKind::Direct,
            bearer_token: "secret".to_string(),
            runtime: None,
            remote_selection: tak_core::model::RemoteSelectionSpec::Shuffle,
            required_pool: None,
            required_tags: Vec::new(),
            required_capabilities: Vec::new(),
            daemon_task_handle: None,
        })
        .collect()
}

pub(super) fn node_ids(targets: &[StrictRemoteTarget]) -> Vec<&str> {
    targets
        .iter()
        .map(|target| target.node_id.as_str())
        .collect()
}

pub(super) fn sorted_node_ids(targets: &[StrictRemoteTarget]) -> Vec<&str> {
    let mut ids = node_ids(targets);
    ids.sort_unstable();
    ids
}
