use tak_proto::worker_v2::{
    ObserveAttemptRequest, WorkerAttemptState, WorkerOutputStream, WorkerTerminalOutcome,
    decode_observe_response, encode_observe_request,
};

use crate::support::{
    worker_http::start_server,
    v2_worker::dispatch,
    v2_worker_http::{post, status},
};

const PAGE_SIZE: usize = 128;

#[tokio::test]
async fn completed_worker_attempt_hides_terminal_state_until_the_last_bounded_event_page() {
    let server = start_server().await;
    let request = dispatch(1, 1, "fence-pages");
    server.store.register_worker_v2_attempt(&request).unwrap();
    server
        .store
        .mark_worker_v2_running(&request.identity)
        .unwrap();
    for _ in 0..=PAGE_SIZE {
        server
            .store
            .append_worker_v2_event(
                &request.identity,
                "//:check",
                WorkerOutputStream::Stdout,
                b"x",
            )
            .unwrap();
    }
    server
        .store
        .complete_worker_v2_attempt(
            &request.identity,
            WorkerTerminalOutcome::Succeeded,
            "5dceaeceb6d8a49d3665594710ace68dfbc1800e2d663d7febb302b4dbee3d27",
        )
        .unwrap();

    let first = observe(&server, &request, 0).await;
    assert_eq!(first.state, WorkerAttemptState::Running);
    assert_eq!(first.events.len(), PAGE_SIZE);
    assert_eq!(first.next_event, PAGE_SIZE as u64);
    assert!(first.terminal.is_none());

    let last = observe(&server, &request, first.next_event).await;
    assert_eq!(last.state, WorkerAttemptState::Completed);
    assert_eq!(last.events.len(), 1);
    assert_eq!(last.next_event, (PAGE_SIZE + 1) as u64);
    assert_eq!(last.terminal.unwrap().event_watermark, last.next_event);
}

async fn observe(
    server: &crate::support::worker_http::RunningServer,
    request: &tak_proto::worker_v2::DispatchAttemptRequest,
    after_event: u64,
) -> tak_proto::worker_v2::ObserveAttemptResponse {
    let body = encode_observe_request(&ObserveAttemptRequest {
        protocol_version: 2,
        identity: request.identity.clone(),
        after_event,
    })
    .unwrap();
    let response = post(
        server,
        "/v2/attempts/observe",
        Some("secret"),
        &["v2"],
        &body,
    )
    .await;
    assert_eq!(status(&response), 200);
    decode_observe_response(&response.body, &request.identity.fencing_token).unwrap()
}
