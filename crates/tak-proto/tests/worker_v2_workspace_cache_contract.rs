use tak_proto::worker_v2::{
    WorkspaceCacheDisposition, WorkspaceCacheProbeRequest, WorkspaceCacheResponse,
    WorkspaceCacheUploadRequest, decode_cache_probe_request, decode_cache_response,
    decode_cache_upload_request, encode_cache_probe_request, encode_cache_response,
    encode_cache_upload_request,
};

use crate::worker_v2_attempt_support::{payload, payload_archive, request};

#[test]
fn worker_cache_protocol_is_strictly_v2_digest_bound_and_reference_only() {
    let descriptor = payload().workspace.descriptor;
    let probe = WorkspaceCacheProbeRequest {
        protocol_version: 2,
        descriptor: descriptor.clone(),
    };
    let encoded = encode_cache_probe_request(&probe).unwrap();
    assert_eq!(decode_cache_probe_request(&encoded).unwrap(), probe);

    let archive = payload_archive();
    let upload = WorkspaceCacheUploadRequest {
        protocol_version: 2,
        descriptor: descriptor.clone(),
        archive_base64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &archive,
        ),
    };
    let encoded = encode_cache_upload_request(&upload).unwrap();
    assert_eq!(decode_cache_upload_request(&encoded).unwrap(), upload);
    let mut corrupt = upload;
    corrupt.archive_base64 = "Y29ycnVwdA==".into();
    assert!(encode_cache_upload_request(&corrupt).is_err());

    let response = WorkspaceCacheResponse {
        protocol_version: 2,
        workspace_fingerprint: descriptor.manifest.fingerprint,
        disposition: WorkspaceCacheDisposition::Stored,
    };
    assert_eq!(
        decode_cache_response(&encode_cache_response(&response).unwrap()).unwrap(),
        response
    );
    let dispatch = serde_json::to_value(request(payload())).unwrap();
    assert!(
        dispatch["payload"]["workspace"]
            .get("archive_base64")
            .is_none()
    );
}
