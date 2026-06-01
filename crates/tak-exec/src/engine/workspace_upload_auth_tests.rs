#![cfg(test)]

use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

use super::remote_models::{RemoteWorkspaceStage, StrictRemoteTarget, StrictRemoteTransportKind};
use super::remote_submit_failure::RemoteSubmitFailureKind;
use super::workspace_upload::upload_workspace_for_submit;
use super::workspace_upload_raw_http_test_support::read_raw_request;

#[tokio::test]
async fn workspace_upload_begin_auth_failure_is_submit_auth_failure() {
    let (addr, server) = spawn_begin_auth_rejecting_server().await;
    let target = direct_target(&addr);
    let workspace = workspace_stage(b"auth rejected archive");

    let result = upload_workspace_for_submit(&target, "run-auth", 1, &workspace, None, None).await;

    let err = result.expect_err("upload begin auth should fail");
    assert_eq!(err.kind, RemoteSubmitFailureKind::Auth);
    server.await.expect("server task");
}

async fn spawn_begin_auth_rejecting_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let request = read_raw_request(&mut stream).await.expect("request");
        assert_eq!(request.path, "/v2/workspaces/uploads/begin");
        stream
            .write_all(
                b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            )
            .await
            .expect("write auth response");
    });
    (addr, server)
}

fn direct_target(addr: &str) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-auth".into(),
        endpoint: format!("http://{addr}"),
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "expired".into(),
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
        manifest_hash: "manifest".into(),
        archive_path,
        archive_byte_len: archive.len() as u64,
        sha256: format!("{:x}", Sha256::digest(archive)),
    }
}
