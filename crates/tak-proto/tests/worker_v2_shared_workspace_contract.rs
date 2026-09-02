use base64::Engine;
use sha2::{Digest, Sha256};
use tak_core::v2::Affinity;
use tak_proto::worker_v2::{
    WorkerWorkspaceReuse, WorkspaceCacheUploadRequest, encode_cache_upload_request,
    encode_dispatch_request,
};

use crate::worker_v2_attempt_support::{payload, request};

#[test]
fn shared_workspace_requires_matching_hard_affinity() {
    let mut payload = shared_payload();
    payload.tasks[0].affinity = None;
    assert!(encode_dispatch_request(&request(payload.clone())).is_err());

    payload.tasks[0].affinity = Some(Affinity::prefer_same_node("shared-group").unwrap());
    assert!(encode_dispatch_request(&request(payload.clone())).is_err());
    payload.tasks[0].affinity = Some(Affinity::require_same_node("different-group").unwrap());
    assert!(encode_dispatch_request(&request(payload.clone())).is_err());
    payload.tasks[0].affinity = Some(Affinity::require_same_node("shared-group").unwrap());
    assert!(encode_dispatch_request(&request(payload)).is_ok());
}

#[test]
fn shared_workspace_preserves_valid_domain_affinity_group_names() {
    let group = "g".repeat(129);
    let mut payload = shared_payload();
    payload.workspace_reuse = WorkerWorkspaceReuse::Shared {
        session_id: "session-a".into(),
        affinity_group: group.clone(),
    };
    payload.tasks[0].affinity = Some(Affinity::require_same_node(group).unwrap());
    assert!(encode_dispatch_request(&request(payload)).is_ok());
}

#[test]
fn shared_workspace_rejects_archive_path_traversal() {
    assert!(encode_cache_upload_request(&cache_upload(traversal_archive())).is_err());
}

#[test]
fn shared_workspace_rejects_escaping_archive_symlinks() {
    assert!(encode_cache_upload_request(&cache_upload(escaping_symlink_archive())).is_err());
}

fn shared_payload() -> tak_proto::worker_v2::WorkerAttemptPayload {
    let mut value = payload();
    value.workspace_reuse = WorkerWorkspaceReuse::Shared {
        session_id: "session-a".into(),
        affinity_group: "shared-group".into(),
    };
    value.tasks[0].affinity = Some(Affinity::require_same_node("shared-group").unwrap());
    value
}

fn cache_upload(archive: Vec<u8>) -> WorkspaceCacheUploadRequest {
    let mut descriptor = payload().workspace.descriptor;
    descriptor.archive_sha256 = format!("{:x}", Sha256::digest(&archive));
    descriptor.archive_size = archive.len() as u64;
    WorkspaceCacheUploadRequest {
        protocol_version: 2,
        descriptor,
        archive_base64: base64::engine::general_purpose::STANDARD.encode(archive),
    }
}

fn traversal_archive() -> Vec<u8> {
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o644);
    header.set_size(0);
    header.set_entry_type(tar::EntryType::Regular);
    let path = b"../outside";
    header.as_mut_bytes()[..path.len()].copy_from_slice(path);
    header.set_cksum();
    let mut archive = header.as_bytes().to_vec();
    archive.extend([0_u8; 1024]);
    archive
}

fn escaping_symlink_archive() -> Vec<u8> {
    let mut archive = Vec::new();
    let mut builder = tar::Builder::new(&mut archive);
    let mut header = tar::Header::new_gnu();
    header.set_mode(0o777);
    header.set_size(0);
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_path("nested/link").unwrap();
    header.set_link_name("../../outside").unwrap();
    header.set_cksum();
    builder.append(&header, std::io::empty()).unwrap();
    builder.finish().unwrap();
    drop(builder);
    archive
}
