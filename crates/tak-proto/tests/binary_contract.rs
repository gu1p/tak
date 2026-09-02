use prost::Message;
use tak_proto::{NodePingResponse, PollTaskEventsResponse, RemoteEvent};

#[test]
fn worker_v2_observations_round_trip_as_binary() {
    let response = PollTaskEventsResponse {
        events: vec![RemoteEvent {
            seq: 7,
            kind: "TASK_STDOUT_CHUNK".into(),
            timestamp_ms: 42,
            chunk_bytes: b"hello\n".to_vec(),
            queue_position: Some(2),
            ..RemoteEvent::default()
        }],
        done: false,
    };
    let encoded = response.encode_to_vec();
    let decoded = PollTaskEventsResponse::decode(encoded.as_slice()).expect("decode events");

    assert!(!decoded.done);
    assert_eq!(decoded.events[0].seq, 7);
    assert_eq!(decoded.events[0].chunk_bytes, b"hello\n");
    assert_eq!(decoded.events[0].queue_position, Some(2));
}

#[test]
fn node_ping_response_round_trips_as_binary() {
    let response = NodePingResponse {
        node_id: "builder-a".to_string(),
        protocol_version: "v2".to_string(),
        health: "healthy".to_string(),
        active_job_count: 2,
        queue_depth: 1,
        resource_summary: "cpu=4 memory=8192MiB".to_string(),
    };

    let encoded = response.encode_to_vec();
    let decoded = NodePingResponse::decode(encoded.as_slice()).expect("decode ping");

    assert_eq!(decoded.node_id, "builder-a");
    assert_eq!(decoded.protocol_version, "v2");
    assert_eq!(decoded.health, "healthy");
    assert_eq!(decoded.active_job_count, 2);
    assert_eq!(decoded.queue_depth, 1);
    assert_eq!(decoded.resource_summary, "cpu=4 memory=8192MiB");
}
