use tak_proto::worker_v2::{
    ObserveAttemptResponse, WorkerAttemptState, WorkerTerminal, WorkerTerminalOutcome,
    decode_observe_response, encode_observe_response,
};

#[test]
fn worker_terminal_runtime_metadata_round_trips_and_is_consistent() {
    let mut response = ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        state: WorkerAttemptState::Completed,
        events: vec![],
        next_event: 0,
        terminal: Some(WorkerTerminal {
            outcome: WorkerTerminalOutcome::Succeeded,
            terminal_digest: "a".repeat(64),
            event_watermark: 0,
            outputs: vec![],
            exit_code: Some(0),
            runtime_kind: Some("containerized".into()),
            runtime_engine: Some("docker".into()),
        }),
    };
    let bytes = encode_observe_response(&response).unwrap();
    assert_eq!(
        decode_observe_response(&bytes, "fence-1").unwrap(),
        response
    );

    let terminal = response.terminal.as_mut().unwrap();
    terminal.runtime_kind = None;
    assert!(encode_observe_response(&response).is_err());
}
