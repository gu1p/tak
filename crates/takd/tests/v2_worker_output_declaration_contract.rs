use tak_core::v2::{OutputSelector, WorkspaceEntry};
use tak_proto::worker_v2::payload_digest;
use takd::SubmitAttemptStore;

use crate::support::v2_worker::dispatch;

#[test]
fn worker_output_publication_is_bound_to_its_task_and_declared_selector() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).unwrap();
    let mut request = dispatch(1, 1, "fence-output");
    request.payload.tasks[0].outputs = vec![OutputSelector::Path {
        value: "declared.txt".into(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    store.register_worker_v2_attempt(&request).unwrap();
    store.mark_worker_v2_running(&request.identity).unwrap();

    assert!(
        store
            .publish_worker_v2_output(
                &request.identity,
                "//:other",
                entry("declared.txt"),
                b"ok\n",
            )
            .is_err()
    );
    assert!(
        store
            .publish_worker_v2_output(
                &request.identity,
                "//:check",
                entry("undeclared.txt"),
                b"ok\n",
            )
            .is_err()
    );
    store
        .publish_worker_v2_output(
            &request.identity,
            "//:check",
            entry("declared.txt"),
            b"ok\n",
        )
        .unwrap();
}

fn entry(path: &str) -> WorkspaceEntry {
    WorkspaceEntry::file(
        path,
        false,
        3,
        "dc51b8c96c2d745df3bd5590d990230a482fd247123599548e0632fdbf97fc22",
    )
    .unwrap()
}
