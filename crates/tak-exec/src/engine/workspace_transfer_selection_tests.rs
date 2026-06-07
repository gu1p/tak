#![cfg(test)]

use super::remote_models::StrictRemoteTransportKind;
use super::workspace_upload::{WorkspaceTransferChoice, selected_workspace_transfer_for_target};
use super::workspace_upload_tor_stream_test_support::{EnvVarGuard, tor_target};

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
fn tor_targets_use_wormhole_with_stream_fallback_by_default() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::remove("TAK_REMOTE_WORKSPACE_TRANSFER");

    assert_eq!(
        selected_workspace_transfer_for_target(&tor_target()).expect("selection"),
        WorkspaceTransferChoice::WormholeWithTorFallback,
    );
}

#[test]
fn transfer_mode_can_force_tor_stream_or_wormhole_required() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "tor-stream");

    assert_eq!(
        selected_workspace_transfer_for_target(&tor_target()).expect("selection"),
        WorkspaceTransferChoice::TorStream,
    );

    drop(_mode);
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

#[test]
fn unsupported_transfer_mode_reports_allowed_values() {
    let _env_lock = super::env_test_lock::env_lock();
    let _mode = EnvVarGuard::set("TAK_REMOTE_WORKSPACE_TRANSFER", "chunks");

    let err = selected_workspace_transfer_for_target(&tor_target()).expect_err("unsupported mode");

    assert!(
        err.message
            .contains("unsupported TAK_REMOTE_WORKSPACE_TRANSFER `chunks`")
    );
    assert!(err.message.contains("tor-stream"));
    assert!(err.message.contains("wormhole-required"));
}
