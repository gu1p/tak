use sha2::{Digest, Sha256};
use tak_core::v2::{Affinity, ContainerSource, Session, SessionReuse, TaskRuntime};

use super::*;
use crate::support::v2_run::scheduler::commit;

#[test]
fn balanced_credits_a_cached_container_image() {
    let (_temp, store, _run_id) = committed_with("image", |request| {
        request.run.tasks[0].runtime = Some(TaskRuntime::container(ContainerSource::Image {
            image: "alpine:3.20".into(),
        }));
    });
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 10),
        SchedulerNode::with_execution_slots("worker-b", 10)
            .with_cached_content("image:alpine:3.20"),
    ];

    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().node_id,
        "worker-b"
    );
}

#[test]
fn balanced_credits_the_candidate_nodes_private_path_snapshot() {
    let (_temp, store, run_id) = committed_with("paths", |request| {
        let affinity = Affinity::prefer_same_node("compiler").unwrap();
        let mut session = Session::new(
            "compiler",
            SessionReuse::Paths {
                paths: vec![tak_core::v2::OutputSelector::Path {
                    value: ".cache".into(),
                }],
            },
            Some(affinity.clone()),
        )
        .unwrap();
        session.id = "compiler".into();
        request.run.jobs[0].session = Some(session);
        request.run.jobs[0].affinity = Some(affinity.clone());
        request.run.tasks[0].affinity = Some(affinity);
    });
    let identity = serde_json::to_vec(&(&run_id, "worker-b", "compiler")).unwrap();
    let path_key = format!("path-cache:{:x}", Sha256::digest(identity));
    let nodes = [
        SchedulerNode::with_execution_slots("worker-a", 10),
        SchedulerNode::with_execution_slots("worker-b", 10).with_cached_content(path_key),
    ];

    assert_eq!(
        store.reserve_next(&nodes).unwrap().unwrap().node_id,
        "worker-b"
    );
}

fn committed_with(
    key: &str,
    customize: impl FnOnce(&mut tak_core::v2::RunSubmission),
) -> (tempfile::TempDir, RunStore, String) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let mut request = independent_jobs(key, 1);
    request.run.jobs[0].placement_policy.selection = RemoteSelection::Balanced;
    customize(&mut request);
    let request = tak_core::v2::RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    let run_id = commit(&store, &request, "uid:1");
    (temp, store, run_id)
}
