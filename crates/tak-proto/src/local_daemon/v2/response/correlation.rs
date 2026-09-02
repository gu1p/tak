use super::Response;

impl Response {
    pub(in crate::local_daemon::v2) fn correlation(&self) -> (&str, u64) {
        match self {
            Self::Error {
                request_id,
                protocol_version,
                ..
            }
            | Self::DaemonStatus {
                request_id,
                protocol_version,
                ..
            }
            | Self::RemotePreview {
                request_id,
                protocol_version,
                ..
            }
            | Self::RemoteAdded {
                request_id,
                protocol_version,
                ..
            }
            | Self::RemoteList {
                request_id,
                protocol_version,
                ..
            }
            | Self::RemoteRemoved {
                request_id,
                protocol_version,
                ..
            }
            | Self::RemoteStatus {
                request_id,
                protocol_version,
                ..
            }
            | Self::RemoteRead {
                request_id,
                protocol_version,
                ..
            }
            | Self::RemoteCandidates {
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
