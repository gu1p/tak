use tak_core::v2::{
    ContainerSource, OutputSelector, PlacementKind, RunSubmission, Session, SessionReuse,
    TaskRuntime,
};
use tak_proto::local_daemon::v2::RunEventKind;
use takd::{AttemptCompletion, AttemptRuntimeMetadata, RunStore, SchedulerNode};

use crate::support::v2_run::{scheduler::commit, submission};

#[test]
fn remote_terminal_metadata_is_durable_for_foreground_and_attach() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let mut request = submission("terminal-metadata", "redacted");
    request.run.tasks[0].runtime = Some(TaskRuntime::container(ContainerSource::Image {
        image: "alpine:3.20".into(),
    }));
    request.run.jobs[0].placement_candidates[0].node_id = "worker-a".into();
    request.run.jobs[0].placement_candidates[0].kind = PlacementKind::Remote;
    request.run.jobs[0].placement_candidates[0].transport = Some("tor".into());
    request.run.jobs[0].placement_candidates[0].reason = "balanced".into();
    request.run.jobs[0].session = Some(
        Session::new(
            "cargo-cache",
            SessionReuse::Paths {
                paths: vec![OutputSelector::Path {
                    value: "target".into(),
                }],
            },
            None,
        )
        .unwrap(),
    );
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let run_id = commit(&store, &request, "alice");
    let command = store
        .reserve_next(&[SchedulerNode::with_execution_slots("worker-a", 1).with_transport("tor")])
        .unwrap()
        .unwrap();
    store
        .complete_attempt(
            &command,
            AttemptCompletion::SucceededWithRuntime {
                terminal_digest: "a".repeat(64),
                runtime: AttemptRuntimeMetadata {
                    kind: "containerized".into(),
                    engine: "docker".into(),
                },
            },
        )
        .unwrap();
    drop(store);

    let events = RunStore::with_db_path(db)
        .unwrap()
        .events_after(&run_id, 0)
        .unwrap();
    let summary = &events
        .iter()
        .find(|event| event.kind == RunEventKind::Succeeded && event.job_id.is_some())
        .unwrap()
        .message;
    for needle in [
        "placement=remote",
        "transport=tor",
        "remote_node=worker-a",
        "reason=balanced",
        "runtime=containerized",
        "runtime_engine=docker",
        "session=cargo-cache",
        "reuse=share_paths",
        "context_hash=",
    ] {
        assert!(summary.contains(needle), "missing `{needle}` in {summary}");
    }
}
