use super::{
    OutputArtifact, RunDetails, RunEvent, RunLifecycleState, RunSummary, WorkspaceDisposition,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum Response {
    #[serde(skip)]
    Error {
        protocol_version: u64,
        request_id: String,
        code: super::DaemonErrorCode,
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

impl Response {
    pub(super) fn correlation(&self) -> (&str, u64) {
        match self {
            Self::Error {
                request_id,
                protocol_version,
                ..
            }
            | Self::RunSubmitted {
                request_id,
                protocol_version,
                ..
            }
            | Self::WorkspaceUploadProgress {
                request_id,
                protocol_version,
                ..
            }
            | Self::RunCommitted {
                request_id,
                protocol_version,
                ..
            }
            | Self::RunList {
                request_id,
                protocol_version,
                ..
            }
            | Self::RunDetails {
                request_id,
                protocol_version,
                ..
            }
            | Self::RunEvents {
                request_id,
                protocol_version,
                ..
            }
            | Self::CancellationAccepted {
                request_id,
                protocol_version,
                ..
            }
            | Self::OutputManifest {
                request_id,
                protocol_version,
                ..
            }
            | Self::OutputChunk {
                request_id,
                protocol_version,
                ..
            } => (request_id, *protocol_version),
        }
    }
}
