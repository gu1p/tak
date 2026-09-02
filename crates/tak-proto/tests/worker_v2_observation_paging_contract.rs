use base64::Engine;
use sha2::{Digest, Sha256};
use tak_proto::worker_v2::{
    ObserveAttemptResponse, WorkerAttemptEvent, WorkerAttemptState, WorkerOutputStream,
    decode_observe_response_page, encode_observe_response,
};

#[test]
fn worker_observation_rejects_oversized_event_pages_and_chunks() {
    let events = (1..=129).map(|seq| event(seq, b"x")).collect();
    assert!(encode_observe_response(&running(events)).is_err());

    let oversized = vec![b'x'; 64 * 1024 + 1];
    assert!(encode_observe_response(&running(vec![event(1, &oversized)])).is_err());

    let skipped = running(vec![event(2, b"x")]);
    let encoded = encode_observe_response(&skipped).unwrap();
    assert!(decode_observe_response_page(&encoded, "fence-1", 0).is_err());

    let gapped = running(vec![event(1, b"x"), event(3, b"x")]);
    let encoded = encode_observe_response(&gapped).unwrap();
    assert!(decode_observe_response_page(&encoded, "fence-1", 0).is_err());
}

fn running(events: Vec<WorkerAttemptEvent>) -> ObserveAttemptResponse {
    ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        next_event: events.last().map_or(0, |event| event.seq),
        state: WorkerAttemptState::Running,
        events,
        terminal: None,
    }
}

fn event(seq: u64, chunk: &[u8]) -> WorkerAttemptEvent {
    WorkerAttemptEvent {
        seq,
        task_id: "//:check".into(),
        stream: WorkerOutputStream::Stdout,
        chunk_base64: base64::engine::general_purpose::STANDARD.encode(chunk),
        chunk_sha256: format!("{:x}", Sha256::digest(chunk)),
    }
}
