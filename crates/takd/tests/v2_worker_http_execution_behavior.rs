use std::time::Duration;

use base64::Engine;
use tak_proto::worker_v2::{
    ObserveAttemptRequest, OutputChunkRequest, WorkerAttemptState, WorkerTerminalOutcome,
    decode_observe_response, decode_output_chunk_response, encode_dispatch_request,
    encode_observe_request, encode_output_chunk_request,
};

use crate::support::{
    worker_http::start_server,
    v2_worker_cache::ensure,
    v2_worker_execution::{output_archive, output_dispatch},
    v2_worker_http::{post, status},
};

#[tokio::test]
async fn worker_v2_dispatch_executes_steps_streams_logs_and_publishes_declared_output() {
    let server = start_server().await;
    let request = output_dispatch();
    ensure(
        &server,
        &request.payload.workspace.descriptor,
        &output_archive(),
    )
    .await;
    let accepted = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &encode_dispatch_request(&request).unwrap(),
    )
    .await;
    assert_eq!(status(&accepted), 202);

    let observed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
        let observe = ObserveAttemptRequest {
            protocol_version: 2,
            identity: request.identity.clone(),
            after_event: 0,
        };
        let response = post(
            &server,
            "/v2/attempts/observe",
            Some("secret"),
            &["v2"],
            &encode_observe_request(&observe).unwrap(),
        )
        .await;
        let observed = decode_observe_response(&response.body, "fence-1").unwrap();
        if observed.state == WorkerAttemptState::Completed {
            break observed;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("worker attempt should finish");
    assert_eq!(observed.events.len(), 1);
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(&observed.events[0].chunk_base64)
            .unwrap(),
        b"hello\n"
    );
    let terminal = observed.terminal.unwrap();
    assert_eq!(terminal.outcome, WorkerTerminalOutcome::Succeeded);
    assert_eq!(terminal.outputs[0].entry.path, "result.txt");
    let output = OutputChunkRequest {
        protocol_version: 2,
        identity: request.identity,
        artifact_id: terminal.outputs[0].artifact_id.clone(),
        offset: 0,
        max_bytes: 32,
    };
    let response = post(
        &server,
        "/v2/attempts/output-chunk",
        Some("secret"),
        &["v2"],
        &encode_output_chunk_request(&output).unwrap(),
    )
    .await;
    let chunk = decode_output_chunk_response(&response.body, &output).unwrap();
    assert!(chunk.eof);
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(chunk.chunk_base64)
            .unwrap(),
        b"ok\n"
    );
}
