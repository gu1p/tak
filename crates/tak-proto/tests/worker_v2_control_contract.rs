use base64::Engine;
use tak_proto::worker_v2::{
    AckAttemptRequest, AckAttemptResponse, CancelAttemptRequest, CancelAttemptResponse,
    CancelDisposition, ObserveAttemptRequest, OutputChunkRequest, OutputChunkResponse,
    WorkerAttemptIdentity, decode_ack_response, decode_cancel_request, decode_cancel_response,
    decode_observe_request, decode_output_chunk_response, encode_ack_response,
    encode_cancel_request, encode_cancel_response, encode_observe_request,
    encode_output_chunk_response,
};

#[test]
fn worker_control_requests_are_strict_v2_and_identity_bound() {
    let observe = ObserveAttemptRequest {
        protocol_version: 2,
        identity: identity(),
        after_event: 7,
    };
    assert_eq!(
        decode_observe_request(&encode_observe_request(&observe).unwrap()).unwrap(),
        observe
    );
    let cancel = CancelAttemptRequest {
        protocol_version: 2,
        identity: identity(),
    };
    assert_eq!(
        decode_cancel_request(&encode_cancel_request(&cancel).unwrap()).unwrap(),
        cancel
    );
    let mut invalid = serde_json::to_value(&cancel).unwrap();
    invalid["protocol_version"] = 1.into();
    assert!(decode_cancel_request(&serde_json::to_vec(&invalid).unwrap()).is_err());
    invalid["protocol_version"] = 2.into();
    invalid["unexpected"] = true.into();
    assert!(decode_cancel_request(&serde_json::to_vec(&invalid).unwrap()).is_err());
}

#[test]
fn worker_control_responses_bind_fence_artifact_cursor_digest_and_bytes() {
    let cancel = CancelAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        disposition: CancelDisposition::Requested,
    };
    assert_eq!(
        decode_cancel_response(&encode_cancel_response(&cancel).unwrap(), "fence-1").unwrap(),
        cancel
    );
    let request = OutputChunkRequest {
        protocol_version: 2,
        identity: identity(),
        artifact_id: "artifact-1".into(),
        offset: 3,
        max_bytes: 8,
    };
    let chunk = OutputChunkResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        artifact_id: "artifact-1".into(),
        offset: 3,
        chunk_base64: base64::engine::general_purpose::STANDARD.encode(b"ok\n"),
        chunk_sha256: "dc51b8c96c2d745df3bd5590d990230a482fd247123599548e0632fdbf97fc22".into(),
        eof: true,
    };
    assert_eq!(
        decode_output_chunk_response(&encode_output_chunk_response(&chunk).unwrap(), &request)
            .unwrap(),
        chunk
    );
    let ack_request = AckAttemptRequest {
        protocol_version: 2,
        identity: identity(),
        terminal_digest: "5dceaeceb6d8a49d3665594710ace68dfbc1800e2d663d7febb302b4dbee3d27".into(),
        run_terminal: false,
    };
    let ack = AckAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        terminal_digest: ack_request.terminal_digest.clone(),
        acknowledged: true,
    };
    assert_eq!(
        decode_ack_response(&encode_ack_response(&ack).unwrap(), &ack_request).unwrap(),
        ack
    );
}

fn identity() -> WorkerAttemptIdentity {
    WorkerAttemptIdentity {
        run_id: "run-1".into(),
        job_id: "job-1".into(),
        node_id: "worker-a".into(),
        authored_attempt: 1,
        dispatch_generation: 1,
        fencing_token: "fence-1".into(),
    }
}
