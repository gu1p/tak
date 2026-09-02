use tak_core::v2::WorkspaceDescriptor;
use tak_proto::worker_v2::{
    WorkspaceCacheDisposition, WorkspaceCacheProbeRequest, WorkspaceCacheResponse,
    WorkspaceCacheUploadRequest, decode_cache_response, encode_cache_probe_request,
    encode_cache_upload_request,
};

use super::v2_worker_http::{post, status};
use super::worker_http::RunningServer;

pub async fn probe(
    server: &RunningServer,
    descriptor: &WorkspaceDescriptor,
) -> WorkspaceCacheResponse {
    let request = WorkspaceCacheProbeRequest {
        protocol_version: 2,
        descriptor: descriptor.clone(),
    };
    let response = post(
        server,
        "/v2/workspaces/cache/probe",
        Some("secret"),
        &["v2"],
        &encode_cache_probe_request(&request).unwrap(),
    )
    .await;
    assert_eq!(status(&response), 200);
    decode_cache_response(&response.body).unwrap()
}

pub async fn upload(
    server: &RunningServer,
    descriptor: &WorkspaceDescriptor,
    archive: &[u8],
) -> WorkspaceCacheResponse {
    let request = WorkspaceCacheUploadRequest {
        protocol_version: 2,
        descriptor: descriptor.clone(),
        archive_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, archive),
    };
    let response = post(
        server,
        "/v2/workspaces/cache/upload",
        Some("secret"),
        &["v2"],
        &encode_cache_upload_request(&request).unwrap(),
    )
    .await;
    assert!(matches!(status(&response), 200 | 201));
    decode_cache_response(&response.body).unwrap()
}

pub async fn ensure(server: &RunningServer, descriptor: &WorkspaceDescriptor, archive: &[u8]) {
    if probe(server, descriptor).await.disposition == WorkspaceCacheDisposition::Miss {
        let disposition = upload(server, descriptor, archive).await.disposition;
        assert!(matches!(
            disposition,
            WorkspaceCacheDisposition::Stored | WorkspaceCacheDisposition::Hit
        ));
    }
}
