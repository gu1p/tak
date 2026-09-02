use tak_core::v2::WorkspaceEntry;
use tak_proto::worker_v2::{
    ObserveAttemptResponse, WorkerAttemptState, WorkerOutputArtifact, WorkerTerminal,
    WorkerTerminalOutcome, encode_observe_response,
};

#[test]
fn completed_worker_observation_cannot_skip_persisted_events() {
    let mut response = completed(WorkerTerminalOutcome::Succeeded);
    response.terminal.as_mut().unwrap().event_watermark = 1;
    assert!(encode_observe_response(&response).is_err());
}

#[test]
fn unsuccessful_worker_terminal_cannot_publish_outputs() {
    let entry = WorkspaceEntry::file(
        "result.txt",
        false,
        0,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    )
    .unwrap();
    for outcome in [
        WorkerTerminalOutcome::Failed,
        WorkerTerminalOutcome::Cancelled,
    ] {
        let mut response = completed(outcome);
        response.terminal.as_mut().unwrap().outputs = vec![WorkerOutputArtifact {
            artifact_id: "artifact-1".into(),
            producer_task_id: "//:check".into(),
            entry: entry.clone(),
        }];
        assert!(encode_observe_response(&response).is_err());
    }
}

fn completed(outcome: WorkerTerminalOutcome) -> ObserveAttemptResponse {
    ObserveAttemptResponse {
        protocol_version: 2,
        fencing_token: "fence-1".into(),
        state: WorkerAttemptState::Completed,
        events: vec![],
        next_event: 0,
        terminal: Some(WorkerTerminal {
            outcome,
            terminal_digest: "a".repeat(64),
            event_watermark: 0,
            outputs: vec![],
            exit_code: None,
            runtime_kind: None,
            runtime_engine: None,
        }),
    }
}
