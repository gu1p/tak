use sha2::{Digest, Sha256};
use tak_core::model::RemoteSelectionSpec;

use super::super::remote_models::{
    RemoteWorkspaceStage, StrictRemoteTarget, StrictRemoteTransportKind,
};

pub(crate) fn tor_target() -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-client-choice".into(),
        endpoint: "http://builder-client-choice.onion".into(),
        transport_kind: StrictRemoteTransportKind::Tor,
        bearer_token: "secret".into(),
        runtime: None,
        remote_selection: RemoteSelectionSpec::Sequential,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
    }
}

pub(crate) fn workspace_stage(archive: &[u8]) -> RemoteWorkspaceStage {
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
