use tak_proto::worker_v2::{
    DispatchDisposition, ObserveAttemptRequest, WorkerAttemptState, decode_dispatch_response,
    decode_observe_response, encode_dispatch_request, encode_observe_request,
};

use crate::support::{
    worker_http::start_server,
    v2_worker::dispatch,
    v2_worker_cache::ensure,
    v2_worker_http::{post, status},
};

#[tokio::test]
async fn authenticated_worker_v2_dispatch_and_observe_are_strict_and_generation_fenced() {
    let server = start_server().await;
    let first = dispatch(1, 1, "fence-1");
    let body = encode_dispatch_request(&first).unwrap();
    let missing = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &[],
        &body,
    )
    .await;
    assert_eq!(status(&missing), 426);
    let duplicated = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2", "v2"],
        &body,
    )
    .await;
    assert_eq!(status(&duplicated), 426);

    ensure(&server, &first.payload.workspace.descriptor, &[]).await;

    let accepted = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &body,
    )
    .await;
    assert_eq!(status(&accepted), 202);
    assert_eq!(
        decode_dispatch_response(&accepted.body, "fence-1")
            .unwrap()
            .disposition,
        DispatchDisposition::Accepted
    );
    let duplicate = post(
        &server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &body,
    )
    .await;
    assert_eq!(status(&duplicate), 200);

    let mut unknown = dispatch(9, 1, "unknown-fence").identity;
    unknown.run_id = "unknown-run".into();
    let observe = ObserveAttemptRequest {
        protocol_version: 2,
        identity: unknown,
        after_event: 7,
    };
    let response = post(
        &server,
        "/v2/attempts/observe",
        Some("secret"),
        &["v2"],
        &encode_observe_request(&observe).unwrap(),
    )
    .await;
    assert_eq!(status(&response), 200);
    let observed = decode_observe_response(&response.body, "unknown-fence").unwrap();
    assert_eq!(observed.state, WorkerAttemptState::Missing);
    assert_eq!(observed.next_event, 7);
}
