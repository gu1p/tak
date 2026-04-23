use std::collections::BTreeMap;

use tak_core::model::{
    ContainerRuntimeSourceSpec, LocalSpec, RemoteRuntimeSpec, RemoteTransportKind,
};

use crate::container_engine::ContainerEngine;

use super::{PlacementMode, RemoteCandidateDiagnostic, RemoteLogChunk, SyncedOutput};

#[derive(Debug, Clone)]
pub(crate) struct StrictRemoteTarget {
    pub(crate) node_id: String,
    pub(crate) endpoint: String,
    pub(crate) transport_kind: StrictRemoteTransportKind,
    pub(crate) bearer_token: String,
    pub(crate) runtime: Option<RemoteRuntimeSpec>,
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
}

#[derive(Debug, Clone)]
pub(crate) struct TaskPlacement {
    pub(crate) placement_mode: PlacementMode,
    pub(crate) remote_node_id: Option<String>,
    pub(crate) strict_remote_target: Option<StrictRemoteTarget>,
    pub(crate) ordered_remote_targets: Vec<StrictRemoteTarget>,
    pub(crate) decision_reason: Option<String>,
    pub(crate) local: Option<LocalSpec>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteProtocolResult {
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) failure_detail: Option<String>,
    pub(crate) synced_outputs: Vec<SyncedOutput>,
    pub(crate) runtime_kind: Option<String>,
    pub(crate) runtime_engine: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedRemoteEvents {
    pub(crate) next_seq: u64,
    pub(crate) done: bool,
    pub(crate) remote_logs: Vec<RemoteLogChunk>,
}

#[derive(Debug)]
pub(crate) struct RemoteWorkspaceStage {
    pub(crate) temp_dir: tempfile::TempDir,
    pub(crate) manifest_hash: String,
    pub(crate) archive_zip_base64: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeExecutionMetadata {
    pub(crate) kind: String,
    pub(crate) engine: Option<String>,
    pub(crate) env_overrides: BTreeMap<String, String>,
    pub(crate) container_plan: Option<ContainerExecutionPlan>,
}

#[derive(Debug, Clone)]
pub(crate) struct ContainerExecutionPlan {
    pub(crate) engine: ContainerEngine,
    pub(crate) source: ContainerRuntimeSourceSpec,
    pub(crate) image: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RemoteSubmitContext<'a> {
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) task_label: &'a str,
    pub(crate) remote_workspace: &'a RemoteWorkspaceStage,
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
