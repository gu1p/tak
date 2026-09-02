use prost::Message;
use tak_proto::worker_v2::WorkerOutputStream;
use tak_proto::{
    ListTaskAttemptsResponse, NodePingResponse, NodeStatusResponse, PollTaskEventsResponse,
};
use takd::SubmitAttemptStore;

#[test]
fn authenticated_worker_v2_observation_routes_surface_v2_attempt_labels_and_logs() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    std::fs::create_dir_all(&state).unwrap();
    std::fs::write(state.join("service.log"), "worker ready\n").unwrap();
    let context = crate::support::remote_output::test_context().with_state_root(&state);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();
    let attempt = crate::support::v2_worker::dispatch(2, 1, "task-run-v2");
    store.register_worker_v2_attempt(&attempt).unwrap();
    store.mark_worker_v2_running(&attempt.identity).unwrap();
    store
        .append_worker_v2_event(
            &attempt.identity,
            "//:check",
            WorkerOutputStream::Stdout,
            b"worker v2 output\n",
        )
        .unwrap();
    let headers = [("X-Tak-Protocol-Version".into(), "v2".into())];

    let status = request(&context, &store, "/v2/worker/status", &headers);
    let status = NodeStatusResponse::decode(unwrap(status).as_slice()).unwrap();
    assert_eq!(status.node.unwrap().node_id, "builder-a");

    let ping = request(&context, &store, "/v2/worker/ping", &headers);
    let ping = NodePingResponse::decode(unwrap(ping).as_slice()).unwrap();
    assert_eq!(ping.node_id, "builder-a");
    assert_eq!(ping.protocol_version, "v2");

    let logs = request(&context, &store, "/v2/worker/logs?all=true", &headers);
    assert_eq!(unwrap(logs), b"worker ready\n");

    let tasks = request(&context, &store, "/v2/worker/tasks?state=all", &headers);
    let tasks = ListTaskAttemptsResponse::decode(unwrap(tasks).as_slice()).unwrap();
    assert_eq!(tasks.attempts.len(), 1);
    assert_eq!(tasks.attempts[0].task_run_id, "task-run-v2");
    assert_eq!(tasks.attempts[0].attempt, 2);
    assert_eq!(tasks.attempts[0].task_label, "//:check");
    assert_eq!(tasks.attempts[0].node_id, "builder-a");

    let events = request(
        &context,
        &store,
        "/v2/worker/tasks/task-run-v2/events?after_seq=0",
        &headers,
    );
    let events = PollTaskEventsResponse::decode(unwrap(events).as_slice()).unwrap();
    assert_eq!(events.events.len(), 1);
    assert_eq!(events.events[0].kind, "TASK_STDOUT_CHUNK");
    assert_eq!(events.events[0].chunk_bytes, b"worker v2 output\n");
}

#[test]
fn worker_v2_observations_reject_missing_version_header() {
    let temp = tempfile::tempdir().unwrap();
    let context = crate::support::remote_output::test_context();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();
    let response = request(&context, &store, "/v2/worker/status", &[]);
    assert_eq!(response.status_code, 426);
    assert!(
        String::from_utf8_lossy(&response.body).contains("upgrade tak, takd, and workers together")
    );
}

fn request(
    context: &takd::RemoteNodeContext,
    store: &SubmitAttemptStore,
    path: &str,
    headers: &[(String, String)],
) -> takd::WorkerHttpResponse {
    takd::daemon::remote::handle_worker_http_request(context, store, "GET", path, headers, None)
        .unwrap()
}

fn unwrap(response: takd::WorkerHttpResponse) -> Vec<u8> {
    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, "application/json");
    tak_proto::worker_v2::decode_display_payload(&response.body).unwrap()
}
