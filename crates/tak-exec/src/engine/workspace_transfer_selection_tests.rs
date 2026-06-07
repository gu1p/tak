#![cfg(test)]

use super::remote_models::StrictRemoteTransportKind;
use super::workspace_upload::{WorkspaceTransferChoice, selected_workspace_transfer_for_target};
use super::workspace_upload_tor_stream_test_support::{tor_target, workspace_stage};

struct EnvVarGuard {
    name: &'static str,
    previous: Option<std::ffi::OsString>,
}

impl EnvVarGuard {
    fn set(name: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(name);
        unsafe { std::env::set_var(name, value) };
        Self { name, previous }
    }

    fn remove(name: &'static str) -> Self {
        let previous = std::env::var_os(name);
        unsafe { std::env::remove_var(name) };
        Self { name, previous }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => unsafe { std::env::set_var(self.name, previous) },
            None => unsafe { std::env::remove_var(self.name) },
        }
    }
}

#[test]
fn direct_targets_keep_chunk_upload_by_default() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::remove("TAK_REMOTE_WORKSPACE_TRANSFER");
    let mut target = tor_target();
    target.transport_kind = StrictRemoteTransportKind::Direct;
    target.endpoint = "http://127.0.0.1:12345".to_string();

    assert_eq!(
        selected_workspace_transfer_for_target(&target).expect("selection"),
        WorkspaceTransferChoice::DirectChunks,
    );
}

#[test]
fn tor_targets_keep_stream_upload_by_default() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::remove("TAK_REMOTE_WORKSPACE_TRANSFER");

    assert_eq!(
        selected_workspace_transfer_for_target(&tor_target()).expect("selection"),
        WorkspaceTransferChoice::TorStream,
    );
}

#[test]
fn wormhole_can_be_selected_as_fallback_or_required() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "wormhole");

    assert_eq!(
        selected_workspace_transfer_for_target(&tor_target()).expect("selection"),
        WorkspaceTransferChoice::WormholeWithTorFallback,
    );

    drop(_mode);
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "wormhole-required");

    assert_eq!(
        selected_workspace_transfer_for_target(&tor_target()).expect("selection"),
        WorkspaceTransferChoice::WormholeRequired,
    );
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn wormhole_falls_back_to_tor_stream_when_remote_route_is_unsupported() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "wormhole");
    let archive = b"fallback archive body".to_vec();
    let daemon =
        super::workspace_upload_tor_stream_test_support::TorStreamUploadDaemon::spawn_with_unsupported_wormhole(&archive)
            .await;
    let workspace = workspace_stage(&archive);

    let outcome = super::workspace_upload::upload_workspace_for_submit(
        &tor_target(),
        "run-1",
        1,
        &workspace,
        None,
        None,
    )
    .await
    .expect("fallback upload");

    assert_eq!(
        outcome.preferred_node_id.as_deref(),
        Some("builder-selected")
    );
    assert_eq!(daemon.wormhole_attempts().await, 1);
    assert_eq!(daemon.bytes().await, archive);
}

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn wormhole_required_does_not_fall_back_to_tor_stream() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "wormhole-required");
    let archive = b"strict archive body".to_vec();
    let daemon =
        super::workspace_upload_tor_stream_test_support::TorStreamUploadDaemon::spawn_with_unsupported_wormhole(&archive)
            .await;
    let workspace = workspace_stage(&archive);

    let err = super::workspace_upload::upload_workspace_for_submit(
        &tor_target(),
        "run-1",
        1,
        &workspace,
        None,
        None,
    )
    .await
    .expect_err("required wormhole failure");

    assert!(err.message.contains("workspace wormhole upload"));
    assert_eq!(daemon.wormhole_attempts().await, 1);
    assert!(daemon.bytes().await.is_empty());
}
