#![cfg(test)]

use std::time::Duration;

use sha2::{Digest, Sha256};

use super::remote_models::{RemoteWorkspaceStage, StrictRemoteTarget, StrictRemoteTransportKind};
use super::workspace_upload::{stream_upload_timeout, upload_workspace_for_submit};
use super::workspace_upload_test_support::UploadServer;

#[tokio::test]
async fn workspace_upload_retries_chunk_after_dropped_response() {
    let server = UploadServer::spawn().await;
    let archive = b"resumable archive body".to_vec();
    let target = direct_target(&server.addr);
    let workspace = workspace_stage(&archive);

    let upload = upload_workspace_for_submit(&target, "run-1", 1, &workspace, None, None)
        .await
        .expect("upload")
        .upload
        .expect("upload route");

    assert_eq!(upload.upload_id, "upload-1");
    assert_eq!(server.bytes().await, archive);
    assert!(server.dropped_response());
}

#[tokio::test]
async fn workspace_upload_resumes_when_finish_reports_incomplete_offset() {
    let server = UploadServer::spawn_finish_conflict(8).await;
    let archive = b"resumable archive body".to_vec();
    let target = direct_target(&server.addr);
    let workspace = workspace_stage(&archive);

    let upload = upload_workspace_for_submit(&target, "run-1", 1, &workspace, None, None)
        .await
        .expect("upload")
        .upload
        .expect("upload route");

    assert_eq!(upload.upload_id, "upload-1");
    assert_eq!(server.bytes().await, archive);
}

#[test]
fn stream_upload_timeout_scales_for_slow_tor_relays() {
    assert_eq!(stream_upload_timeout(0), Duration::from_secs(120));
    assert!(stream_upload_timeout(12 * 1024 * 1024) >= Duration::from_secs(600));
}

fn direct_target(addr: &str) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: format!("http://{addr}"),
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
        remote_selection: tak_core::model::RemoteSelectionSpec::Sequential,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}

fn workspace_stage(archive: &[u8]) -> RemoteWorkspaceStage {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let archive_path = temp_dir.path().join("workspace.zip");
    std::fs::write(&archive_path, archive).expect("archive");
    RemoteWorkspaceStage {
        temp_dir,
        archive_path,
        archive_byte_len: archive.len() as u64,
        sha256: format!("{:x}", Sha256::digest(archive)),
    }
}
