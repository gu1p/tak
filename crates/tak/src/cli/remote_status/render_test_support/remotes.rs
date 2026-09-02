use crate::cli::remote_inventory::RemoteRecord;
use crate::cli::remote_status::RemoteStatusResult;

use super::status::status;

pub(in super::super) fn remote(node_id: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.to_string(),
        display_name: node_id.to_string(),
        base_url: format!("http://{node_id}.example"),
        pools: vec!["default".to_string()],
        tags: vec!["builder".to_string()],
        capabilities: vec!["linux".to_string()],
        transport: "direct".to_string(),
        enabled: true,
    }
}

pub(in super::super) fn ok_result(node_id: &str, with_job: bool) -> RemoteStatusResult {
    RemoteStatusResult {
        remote: remote(node_id),
        status: Some(status(node_id, "ready", with_job)),
        error: None,
        peer: None,
    }
}

pub(in super::super) fn warning_result(node_id: &str) -> RemoteStatusResult {
    RemoteStatusResult {
        remote: remote(node_id),
        status: Some(status(node_id, "recovering", false)),
        error: None,
        peer: None,
    }
}

pub(in super::super) fn error_result(node_id: &str) -> RemoteStatusResult {
    RemoteStatusResult {
        remote: remote(node_id),
        status: None,
        error: Some("node status failed with HTTP 401".to_string()),
        peer: None,
    }
}
