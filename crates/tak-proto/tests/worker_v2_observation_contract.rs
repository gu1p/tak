use tak_core::v2::WorkspaceEntry;
use tak_proto::worker_v2::{
    DispatchAttemptResponse, DispatchDisposition, ObserveAttemptResponse, WorkerAttemptEvent,
    WorkerAttemptState, WorkerOutputArtifact, WorkerOutputStream, WorkerTerminal,
    WorkerTerminalOutcome, decode_dispatch_response, decode_observe_response,
    encode_dispatch_response, encode_observe_response,
};

#[test]
fn worker_dispatch_and_terminal_observation_are_fence_correlated_and_strict() {
    let dispatch = DispatchAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        disposition: DispatchDisposition::Accepted,
    };
    assert_eq!(
        decode_dispatch_response(&encode_dispatch_response(&dispatch).unwrap(), "fence-1").unwrap(),
        dispatch
    );
    assert!(
        decode_dispatch_response(&encode_dispatch_response(&dispatch).unwrap(), "other").is_err()
    );

    let observed = ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        state: WorkerAttemptState::Completed,
        events: vec![WorkerAttemptEvent {
            seq: 1,
            task_id: "//:check".into(),
            stream: WorkerOutputStream::Stdout,
            chunk_base64: "b2sK".into(),
            chunk_sha256: "dc51b8c96c2d745df3bd5590d990230a482fd247123599548e0632fdbf97fc22".into(),
        }],
        next_event: 1,
        terminal: Some(WorkerTerminal {
            outcome: WorkerTerminalOutcome::Succeeded,
            terminal_digest: "5dceaeceb6d8a49d3665594710ace68dfbc1800e2d663d7febb302b4dbee3d27"
                .into(),
            event_watermark: 1,
            outputs: vec![],
            exit_code: None,
            runtime_kind: None,
            runtime_engine: None,
        }),
    };
    assert_eq!(
        decode_observe_response(&encode_observe_response(&observed).unwrap(), "fence-1").unwrap(),
        observed
    );
    let mut invalid = observed;
    invalid.terminal.as_mut().unwrap().event_watermark = 0;
    assert!(encode_observe_response(&invalid).is_err());
}

#[test]
fn fused_worker_outputs_are_canonical_per_producer_before_origin_precedence_resolution() {
    let entry = WorkspaceEntry::file(
        "result.txt",
        false,
        0,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    )
    .unwrap();
    let mut observed = ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        state: WorkerAttemptState::Completed,
        events: vec![],
        next_event: 0,
        terminal: Some(WorkerTerminal {
            outcome: WorkerTerminalOutcome::Succeeded,
            terminal_digest: "5dceaeceb6d8a49d3665594710ace68dfbc1800e2d663d7febb302b4dbee3d27"
                .into(),
            event_watermark: 0,
            outputs: vec![
                WorkerOutputArtifact {
                    artifact_id: "artifact-a".into(),
                    producer_task_id: "//:first".into(),
                    entry: entry.clone(),
                },
                WorkerOutputArtifact {
                    artifact_id: "artifact-b".into(),
                    producer_task_id: "//:second".into(),
                    entry,
                },
            ],
            exit_code: None,
            runtime_kind: None,
            runtime_engine: None,
        }),
    };
    assert!(encode_observe_response(&observed).is_ok());
    observed.terminal.as_mut().unwrap().outputs.reverse();
    assert!(encode_observe_response(&observed).is_err());
}
