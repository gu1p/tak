use tak_proto::worker_v2::{
    WorkerTerminalOutcome, decode_observe_response, encode_observe_response,
};

#[test]
fn failed_worker_terminal_round_trips_the_exact_process_exit_code() {
    let raw = br#"{"protocol_version":2,"fencing_token":"fence-1","state":"completed","events":[],"next_event":0,"terminal":{"outcome":"failed","terminal_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","event_watermark":0,"outputs":[],"exit_code":7}}"#;
    let observed = decode_observe_response(raw, "fence-1").unwrap();
    let terminal = observed.terminal.as_ref().unwrap();
    assert_eq!(terminal.outcome, WorkerTerminalOutcome::Failed);
    assert_eq!(terminal.exit_code, Some(7));
    assert!(encode_observe_response(&observed).is_ok());
}
