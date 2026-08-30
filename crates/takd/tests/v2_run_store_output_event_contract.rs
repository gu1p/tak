use tak_proto::local_daemon::v2::RunEventKind;
use takd::{AttemptCompletion, AttemptOutputStream, ResultAcceptance, RunStore, SchedulerNode};

use crate::support::v2_run::{scheduler::commit, submission};

#[test]
fn attempt_output_is_persisted_in_order_and_fenced() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &submission("output-events", "secret"), "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("local", 1)])
        .unwrap()
        .unwrap();
    assert_eq!(
        store
            .append_attempt_output(
                &command,
                "//:check",
                AttemptOutputStream::Stdout,
                &[0, 255, 10]
            )
            .unwrap(),
        ResultAcceptance::Applied
    );
    assert!(
        store
            .append_attempt_output(&command, "//:forged", AttemptOutputStream::Stdout, b"")
            .is_err()
    );
    store.ack_dispatch(&command).unwrap();
    store
        .append_attempt_output(
            &command,
            "//:check",
            AttemptOutputStream::Stderr,
            b"error\n",
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
    assert_eq!(
        store
            .append_attempt_output(&command, "//:check", AttemptOutputStream::Stdout, b"late")
            .unwrap(),
        ResultAcceptance::Stale
    );

    let restored = RunStore::with_db_path(db).unwrap();
    let events = restored.events_after(&run_id, 0).unwrap();
    let logs = events
        .iter()
        .filter(|event| matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr))
        .collect::<Vec<_>>();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].chunk_base64.as_deref(), Some("AP8K"));
    assert_eq!(logs[1].chunk_base64.as_deref(), Some("ZXJyb3IK"));
    assert!(logs.iter().all(|event| event.task_ids == ["//:check"]));
    assert!(
        logs.iter()
            .all(|event| event.job_id.as_deref() == Some("job-0"))
    );
}
