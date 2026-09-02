use tak_proto::worker_v2::{
    AckAttemptRequest, CancelAttemptRequest, ObserveAttemptRequest, OutputChunkRequest,
    encode_ack_request, encode_cancel_request, encode_dispatch_request, encode_observe_request,
    encode_output_chunk_request,
};

use crate::support::{
    v2_worker_execution::output_dispatch,
    v2_worker_http::{post, status},
    worker_http::start_server,
};

#[tokio::test]
async fn worker_v2_rejects_a_forged_node_identity_on_every_attempt_route() {
    let server = start_server().await;
    let mut request = output_dispatch();
    request.identity.node_id = "forged-node".into();
    let identity = request.identity.clone();
    let cases = vec![
        (
            "/v2/attempts/dispatch",
            encode_dispatch_request(&request).unwrap(),
        ),
        (
            "/v2/attempts/observe",
            encode_observe_request(&ObserveAttemptRequest {
                protocol_version: 2,
                identity: identity.clone(),
                after_event: 0,
            })
            .unwrap(),
        ),
        (
            "/v2/attempts/cancel",
            encode_cancel_request(&CancelAttemptRequest {
                protocol_version: 2,
                identity: identity.clone(),
            })
            .unwrap(),
        ),
        (
            "/v2/attempts/output-chunk",
            encode_output_chunk_request(&OutputChunkRequest {
                protocol_version: 2,
                identity: identity.clone(),
                artifact_id: "artifact-1".into(),
                offset: 0,
                max_bytes: 1,
            })
            .unwrap(),
        ),
        (
            "/v2/attempts/ack",
            encode_ack_request(&AckAttemptRequest {
                protocol_version: 2,
                identity,
                terminal_digest: "a".repeat(64),
                run_terminal: false,
            })
            .unwrap(),
        ),
    ];
    for (path, body) in cases {
        let response = post(&server, path, Some("secret"), &["v2"], &body).await;
        assert_eq!(status(&response), 409, "{path} accepted a foreign node id");
        assert!(String::from_utf8_lossy(&response.body).contains("node_identity_mismatch"));
    }
}
