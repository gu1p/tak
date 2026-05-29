#![cfg(test)]

use base64::Engine;

use super::remote_models::{RemoteWorkspaceStage, StrictRemoteTarget, StrictRemoteTransportKind};
use super::workspace_upload::upload_workspace_for_submit;
use super::workspace_upload_test_support::UploadServer;

#[tokio::test]
async fn workspace_upload_retries_chunk_after_dropped_response() {
    let server = UploadServer::spawn().await;
    let archive = b"resumable archive body".to_vec();
    let target = direct_target(&server.addr);
    let workspace = workspace_stage(&archive);

    let upload = upload_workspace_for_submit(&target, "run-1", 1, &workspace)
        .await
        .expect("upload")
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

    let upload = upload_workspace_for_submit(&target, "run-1", 1, &workspace)
        .await
        .expect("upload")
        .expect("upload route");

    assert_eq!(upload.upload_id, "upload-1");
    assert_eq!(server.bytes().await, archive);
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
    RemoteWorkspaceStage {
        temp_dir: tempfile::tempdir().expect("tempdir"),
        manifest_hash: "manifest".into(),
        archive_zip_base64: base64::engine::general_purpose::STANDARD.encode(archive),
        archive_byte_len: archive.len(),
    }
}
