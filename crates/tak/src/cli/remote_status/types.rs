use serde::Deserialize;

use super::RemoteRecord;

#[derive(Clone, Debug)]
pub(in crate::cli) struct RemoteStatusResult {
    pub(in crate::cli) remote: RemoteRecord,
    pub(in crate::cli) status: Option<tak_proto::NodeStatusResponse>,
    pub(in crate::cli) error: Option<String>,
    pub(in crate::cli) peer: Option<DaemonPeerSnapshot>,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::cli) struct DaemonPeerSnapshot {
    pub(in crate::cli) node_id: String,
    #[serde(default)]
    pub(in crate::cli) display_name: String,
    pub(in crate::cli) transport: String,
    pub(in crate::cli) endpoint: String,
    pub(in crate::cli) state: String,
    #[serde(default)]
    pub(in crate::cli) last_heartbeat_ms: Option<i64>,
    #[serde(default)]
    pub(in crate::cli) last_successful_connection_ms: Option<i64>,
    #[serde(default)]
    pub(in crate::cli) last_error_summary: Option<String>,
    #[serde(default)]
    pub(in crate::cli) active_job_count: Option<u32>,
    #[serde(default)]
    pub(in crate::cli) queue_depth: Option<u32>,
    #[serde(default)]
    pub(in crate::cli) resource_summary: Option<String>,
    #[serde(default)]
    pub(in crate::cli) protocol_version: Option<String>,
    #[serde(default)]
    pub(in crate::cli) heartbeat_rtt_ms: Option<u64>,
    #[serde(default)]
    pub(in crate::cli) reconnect_attempts: u32,
    #[serde(default)]
    pub(in crate::cli) pools: Vec<String>,
    #[serde(default)]
    pub(in crate::cli) tags: Vec<String>,
    #[serde(default)]
    pub(in crate::cli) capabilities: Vec<String>,
}
