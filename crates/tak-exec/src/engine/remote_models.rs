use std::collections::BTreeMap;
use std::path::PathBuf;

use tak_core::model::{
    ContainerRuntimeSourceSpec, LocalSpec, RemoteRuntimeSpec, RemoteSelectionSpec, RemoteSpec,
    RemoteTransportKind, ResolvedTask, SessionUseSpec, TaskLabel,
};

use crate::ImageCacheOptions;
use crate::container_engine::ContainerEngine;

use super::{PlacementMode, RemoteCandidateDiagnostic, RemoteLogChunk, SyncedOutput};

const DAEMON_TOR_PLACEMENT_NODE_ID: &str = "__takd_daemon_tor__";
const DAEMON_TOR_PLACEMENT_ENDPOINT: &str = "http://takd-daemon-placement.onion";

#[derive(Debug, Clone)]
pub(crate) struct StrictRemoteTarget {
    pub(crate) node_id: String,
    pub(crate) endpoint: String,
    pub(crate) transport_kind: StrictRemoteTransportKind,
    pub(crate) bearer_token: String,
    pub(crate) runtime: Option<RemoteRuntimeSpec>,
    pub(crate) remote_selection: RemoteSelectionSpec,
    pub(crate) required_pool: Option<String>,
    pub(crate) required_tags: Vec<String>,
    pub(crate) required_capabilities: Vec<String>,
    pub(crate) daemon_task_handle: Option<String>,
}

impl StrictRemoteTarget {
    pub(crate) fn daemon_tor_placement(remote: &RemoteSpec) -> Self {
        Self {
            node_id: DAEMON_TOR_PLACEMENT_NODE_ID.to_string(),
            endpoint: DAEMON_TOR_PLACEMENT_ENDPOINT.to_string(),
            transport_kind: StrictRemoteTransportKind::Tor,
            bearer_token: String::new(),
            runtime: remote.runtime.clone(),
            remote_selection: remote.selection,
            required_pool: remote.pool.clone(),
            required_tags: remote.required_tags.clone(),
            required_capabilities: remote.required_capabilities.clone(),
            daemon_task_handle: None,
        }
    }

    pub(crate) fn is_daemon_tor_placement(&self) -> bool {
        self.transport_kind == StrictRemoteTransportKind::Tor
            && self.node_id == DAEMON_TOR_PLACEMENT_NODE_ID
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrictRemoteTransportKind {
    Direct,
    Tor,
}

impl StrictRemoteTransportKind {
    pub(crate) fn from_inventory_value(value: &str) -> Option<Self> {
        match value {
            "direct" => Some(Self::Direct),
            "tor" => Some(Self::Tor),
            _ => None,
        }
    }

    pub(crate) fn matches_requested(self, requested: RemoteTransportKind) -> bool {
        requested == RemoteTransportKind::Any || self.as_requested_kind() == requested
    }

    pub(crate) fn as_result_value(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Tor => "tor",
        }
    }

    fn as_requested_kind(self) -> RemoteTransportKind {
        match self {
            Self::Direct => RemoteTransportKind::Direct,
            Self::Tor => RemoteTransportKind::Tor,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteTargetSelection {
    pub(crate) configured_remote_count: usize,
    pub(crate) enabled_remote_count: usize,
    pub(crate) enabled_remotes: Vec<RemoteCandidateDiagnostic>,
    pub(crate) matched_targets: Vec<StrictRemoteTarget>,
    pub(crate) matched_tor_target_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskPlacement {
    pub(crate) placement_mode: PlacementMode,
    pub(crate) remote_node_id: Option<String>,
    pub(crate) strict_remote_target: Option<StrictRemoteTarget>,
    pub(crate) ordered_remote_targets: Vec<StrictRemoteTarget>,
    pub(crate) remote_selection: RemoteSelectionSpec,
    pub(crate) decision_reason: Option<String>,
    pub(crate) local: Option<LocalSpec>,
    pub(crate) remote: Option<RemoteSpec>,
    pub(crate) session: Option<SessionUseSpec>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteProtocolResult {
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) failure_detail: Option<String>,
    pub(crate) synced_outputs: Vec<SyncedOutput>,
    pub(crate) runtime_kind: Option<String>,
    pub(crate) runtime_engine: Option<String>,
    pub(crate) stdout_tail: Option<String>,
    pub(crate) stderr_tail: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedRemoteEvents {
    pub(crate) next_seq: u64,
    pub(crate) done: bool,
    pub(crate) remote_logs: Vec<RemoteLogChunk>,
    pub(crate) status_messages: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct RemoteWorkspaceStage {
    pub(crate) temp_dir: tempfile::TempDir,
    pub(crate) manifest_hash: String,
    pub(crate) archive_path: PathBuf,
    pub(crate) archive_byte_len: u64,
    pub(crate) sha256: String,
}

impl RemoteWorkspaceStage {
    pub(crate) fn upload_size_mb(&self) -> String {
        format_upload_size_mb(self.archive_byte_len)
    }
}

pub(crate) fn format_upload_size_mb(byte_len: u64) -> String {
    format!("{:.2} MB", byte_len as f64 / 1_000_000.0)
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionMetadata {
    pub(crate) kind: String,
    pub(crate) engine: Option<String>,
    pub(crate) env_overrides: BTreeMap<String, String>,
    pub(crate) container_plan: Option<ContainerExecutionPlan>,
    pub(crate) container_identity: Option<super::ContainerExecutionIdentity>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContainerExecutionPlan {
    pub(crate) engine: ContainerEngine,
    pub(crate) source: ContainerRuntimeSourceSpec,
    pub(crate) image: String,
    pub(crate) container_user: Option<String>,
    pub(crate) image_cache: Option<ImageCachePlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageCachePlan {
    pub(crate) options: ImageCacheOptions,
    pub(crate) cache_key: String,
    pub(crate) source_kind: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RemoteSubmitContext<'a> {
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) remote_workspace: &'a RemoteWorkspaceStage,
    pub(crate) session: Option<&'a super::session_workspaces::PreparedTaskSession>,
    pub(crate) fused_members: Option<&'a [ResolvedTask]>,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) fused_member_execution_labels: Option<&'a BTreeMap<TaskLabel, String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainerLifecycleStage {
    Pull,
    Start,
    Runtime,
}

impl ContainerLifecycleStage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pull => "pull",
            Self::Start => "start",
            Self::Runtime => "runtime",
        }
    }
}
