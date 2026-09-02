use super::{
    DaemonStatusSnapshot, OutputArtifact, RemoteInventoryEntry, RemoteStatusEntry, RunDetails,
    RunEvent, RunLifecycleState, RunSummary, WorkspaceDisposition,
};
use serde::{Deserialize, Serialize};
use tak_core::v2::PlacementCandidate;

#[path = "response/correlation.rs"]
mod correlation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Response {
    #[serde(skip)]
    Error {
        protocol_version: u64,
        request_id: String,
        code: super::DaemonErrorCode,
    },
    DaemonStatus {
        protocol_version: u64,
        request_id: String,
        status: DaemonStatusSnapshot,
    },
    RemotePreview {
        protocol_version: u64,
        request_id: String,
        remote: RemoteInventoryEntry,
    },
    RemoteAdded {
        protocol_version: u64,
        request_id: String,
        remote: RemoteInventoryEntry,
    },
    RemoteList {
        protocol_version: u64,
        request_id: String,
        remotes: Vec<RemoteInventoryEntry>,
    },
    RemoteRemoved {
        protocol_version: u64,
        request_id: String,
        node_id: String,
        removed: bool,
    },
    RemoteStatus {
        protocol_version: u64,
        request_id: String,
        remotes: Vec<RemoteStatusEntry>,
    },
    RemoteRead {
        protocol_version: u64,
        request_id: String,
        node_id: String,
        http_status: u16,
        body_base64: String,
    },
    RemoteCandidates {
        protocol_version: u64,
        request_id: String,
        candidates: Vec<PlacementCandidate>,
    },
    RunSubmitted {
        protocol_version: u64,
        request_id: String,
        run_id: String,
        workspace: WorkspaceDisposition,
    },
    WorkspaceUploadProgress {
        protocol_version: u64,
        request_id: String,
        run_id: String,
        workspace_fingerprint: String,
        chunk_accepted: bool,
        next_offset: u64,
        complete: bool,
    },
    RunCommitted {
        protocol_version: u64,
        request_id: String,
        run_id: String,
        state: RunLifecycleState,
    },
    RunList {
        protocol_version: u64,
        request_id: String,
        runs: Vec<RunSummary>,
    },
    RunDetails {
        protocol_version: u64,
        request_id: String,
        run: RunDetails,
    },
    RunEvents {
        protocol_version: u64,
        request_id: String,
        run_id: String,
        events: Vec<RunEvent>,
        next_event: u64,
        state: RunLifecycleState,
        terminal: bool,
        #[serde(default)]
        logs_expired: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
    },
    CancellationAccepted {
        protocol_version: u64,
        request_id: String,
        run_id: String,
        state: RunLifecycleState,
    },
    OutputManifest {
        protocol_version: u64,
        request_id: String,
        run_id: String,
        expired: bool,
        artifacts: Vec<OutputArtifact>,
    },
    OutputChunk {
        protocol_version: u64,
        request_id: String,
        artifact_id: String,
        offset: u64,
        chunk_base64: String,
        complete: bool,
    },
}
