use tak_proto::worker_v2::decode_ack_request;

#[test]
fn terminal_ack_explicitly_signals_when_the_daemon_has_settled_the_run() {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "protocol_version": 2,
        "identity": {
            "run_id": "run-a",
            "job_id": "job-a",
            "node_id": "worker-a",
            "authored_attempt": 1,
            "dispatch_generation": 1,
            "fencing_token": "fence-a"
        },
        "terminal_digest": "a".repeat(64),
        "run_terminal": true
    }))
    .unwrap();

    let decoded = decode_ack_request(&bytes).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap()["run_terminal"], true);
}
