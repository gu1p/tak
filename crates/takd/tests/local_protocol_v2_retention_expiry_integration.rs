use takd::{AttemptCompletion, AttemptOutputStream, RunStore, SchedulerNode};

use crate::support::raw_local_protocol::RawLocalProtocol;
use crate::support::v2_run::{scheduler::commit, submission};

#[tokio::test(flavor = "multi_thread")]
async fn local_protocol_exposes_expiration_without_returning_retained_payloads() {
    let root = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(root.path().join("state/takd.sqlite")).unwrap();
    let run_id = commit(&store, &submission("protocol-expiry", "secret"), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    store
        .append_attempt_output(
            &command,
            "//:check",
            AttemptOutputStream::Stdout,
            b"retained-secret-log",
        )
        .unwrap();
    store
        .complete_attempt(
            &command,
            AttemptCompletion::Succeeded {
                terminal_digest: "a".repeat(64),
            },
        )
        .unwrap();
    store.expire_run_payloads(&run_id).unwrap();
    drop(store);

    let mut daemon = RawLocalProtocol::start_in(root.path()).await;
    let details = exchange(&mut daemon, "show", "GetRun", &run_id).await;
    assert_eq!(details["run"]["logs_expired"], true);
    assert_eq!(details["run"]["outputs_expired"], true);
    let attached = daemon
        .exchange(&format!(r#"{{"protocol_version":2,"request_id":"attach","operation":{{"type":"AttachRun","run_id":"{run_id}","after_event":0}}}}"#))
        .await;
    assert!(!attached.contains("retained-secret-log"));
    let attached: serde_json::Value = serde_json::from_str(&attached).unwrap();
    assert_eq!(attached["logs_expired"], true);
    assert!(
        attached["events"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["kind"] != "stdout" && event["kind"] != "stderr")
    );
    let outputs = exchange(&mut daemon, "outputs", "GetOutputManifest", &run_id).await;
    assert_eq!(outputs["expired"], true);
    assert_eq!(outputs["artifacts"], serde_json::json!([]));
}

async fn exchange(
    daemon: &mut RawLocalProtocol,
    request_id: &str,
    operation: &str,
    run_id: &str,
) -> serde_json::Value {
    let raw = daemon
        .exchange(&format!(r#"{{"protocol_version":2,"request_id":"{request_id}","operation":{{"type":"{operation}","run_id":"{run_id}"}}}}"#))
        .await;
    serde_json::from_str(&raw).unwrap()
}
