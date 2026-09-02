use tak_proto::worker_v2::{
    ObserveAttemptResponse, WorkerAttemptEvent, WorkerAttemptState, WorkerOutputStream,
    encode_observe_response,
};

#[test]
fn missing_attempt_retains_durable_events_for_origin_reconciliation() {
    let observed = ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        state: WorkerAttemptState::Missing,
        events: vec![WorkerAttemptEvent {
            seq: 1,
            task_id: "//:check".into(),
            stream: WorkerOutputStream::Stdout,
            chunk_base64: "b2sK".into(),
            chunk_sha256: "dc51b8c96c2d745df3bd5590d990230a482fd247123599548e0632fdbf97fc22".into(),
        }],
        next_event: 1,
        terminal: None,
    };
    assert!(encode_observe_response(&observed).is_ok());
}
