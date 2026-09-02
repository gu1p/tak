use serde::Serialize;
use tak_core::v2::{EnvironmentValue, RemoteRequirements, ResolvedRun};

#[derive(Serialize)]
#[serde(tag = "type")]
pub(super) enum WireOperation<'a> {
    GetDaemonStatus {},
    PreviewRemote {
        invite: &'a str,
    },
    AddRemote {
        invite: &'a str,
    },
    ListRemotes {},
    RemoveRemote {
        node_id: &'a str,
    },
    GetRemoteStatus {
        node_ids: &'a [String],
    },
    ReadRemote {
        node_id: &'a str,
        path: &'a str,
    },
    ResolveRemoteCandidates {
        requirements: &'a RemoteRequirements,
    },
    SubmitRun {
        idempotency_key: &'a str,
        run: &'a ResolvedRun,
        environment_values: &'a [EnvironmentValue],
    },
    UploadWorkspace {
        run_id: &'a str,
        workspace_fingerprint: &'a str,
        archive_size: u64,
        offset: u64,
        chunk_base64: String,
    },
    CommitRun {
        run_id: &'a str,
    },
    ListRuns {},
    GetRun {
        run_id: &'a str,
    },
    AttachRun {
        run_id: &'a str,
        after_event: u64,
    },
    CancelRun {
        run_id: &'a str,
    },
    GetOutputManifest {
        run_id: &'a str,
    },
    GetOutputChunk {
        artifact_id: &'a str,
        offset: u64,
        max_bytes: u32,
    },
}
