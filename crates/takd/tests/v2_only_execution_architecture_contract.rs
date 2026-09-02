use std::path::Path;

#[test]
fn daemon_has_no_v1_client_or_worker_execution_adapter() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let types = read(root, "src/daemon/protocol/types.rs");
    let local_protocol = read(root, "src/daemon/protocol/local_protocol_io.rs");
    let broker = read(root, "src/daemon/protocol/broker/mod.rs");
    let router = read(root, "src/daemon/remote/router.rs");
    let runtime_services = read(root, "src/daemon/remote/runtime_services.rs");
    let remote = read(root, "src/daemon/remote/mod.rs");
    let context = read(root, "src/daemon/remote/types.rs");
    let admission = read(root, "src/daemon/remote/resource_admission/operations.rs");
    let status = read(root, "src/daemon/remote/status_state.rs");
    let runtime = read(root, "src/daemon/remote/runtime.rs");
    let broker_response = read(root, "src/daemon/protocol/broker/tor_client/types.rs");

    for removed in [
        "PlaceRemoteRequest",
        "PeersEligibleRequest",
        "ForwardRemoteHttpRequest",
        "StreamTaskEventsRequest",
        "CancelTaskRequest",
        "GetTaskResultRequest",
        "GetOutputRangeRequest",
        "preferred_node_id",
    ] {
        assert!(!types.contains(removed), "legacy type remains: {removed}");
    }
    assert_absent(
        &types,
        &["pub enum Request", "pub enum Response", "PeersListRequest"],
    );
    assert_absent(
        &local_protocol,
        &[
            "decode_and_dispatch_legacy",
            "ProtocolResponse::Legacy",
            "handle_broker_http_request",
        ],
    );
    assert_absent(
        &broker,
        &[
            "handle_broker_http_request",
            "X-Tak-Remote-Endpoint",
            "LocalBrokerRequest",
        ],
    );
    assert!(!router.contains("handle_remote_submit_route"));
    assert!(!router.contains("handle_remote_cancel_route"));
    assert!(!runtime_services.contains("spawn_remote_orphan_watchdog"));
    assert!(router.contains("reject_v1_route"));
    assert_absent(&remote, &["mod active_executions", "status_job_metadata"]);
    assert_absent(
        &context,
        &[
            "active_executions",
            "admit_or_queue_resources",
            "wait_until_resources_admitted_with_positions",
            "register_active_job",
        ],
    );
    assert_absent(
        &admission,
        &["admit_or_queue(", "wait_until_admitted_with_positions"],
    );
    assert_absent(
        &status,
        &["register_job(", "finish_job(", "update_job_label("],
    );
    assert_absent(
        &runtime,
        &["remote_client_stale_ttl", "remote_client_watchdog_interval"],
    );
    assert!(!broker_response.contains("pub status: u16,\n    pub headers"));
}

fn assert_absent(source: &str, removed: &[&str]) {
    for symbol in removed {
        assert!(!source.contains(symbol), "legacy symbol remains: {symbol}");
    }
}

fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).expect(relative)
}
