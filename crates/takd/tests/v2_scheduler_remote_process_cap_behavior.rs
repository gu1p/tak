use rusqlite::Connection;
use tak_core::v2::LimiterDefinition;
use tak_proto::worker_v2::WorkerProcessObservation;
use takd::{RunStore, SchedulerNode};

use crate::support::{
    v2_node_loss,
    v2_run::{
        constraints::project_process_cap,
        scheduler::{commit, independent_jobs},
    },
};

#[test]
fn remote_process_caps_use_the_worker_snapshot_instead_of_origin_processes() {
    std::fs::create_dir_all(".tmp").unwrap();
    let origin_marker = format!("tak-origin-process-{}", uuid::Uuid::new_v4());
    let mut external = std::process::Command::new("/bin/sh")
        .args(["-c", "trap ':' EXIT; sleep 30", &origin_marker])
        .spawn()
        .unwrap();
    let (_origin_root, origin_store) = store_with_process_pattern("origin-only", origin_marker);
    assert!(
        origin_store
            .reserve_next(&remote_nodes(Vec::new()))
            .unwrap()
            .is_some(),
        "an origin process must not consume a remote node process cap"
    );
    external.kill().unwrap();
    external.wait().unwrap();

    let remote_marker = format!("tak-remote-process-{}", uuid::Uuid::new_v4());
    let (_remote_root, remote_store) =
        store_with_process_pattern("remote-only", remote_marker.clone());
    let processes = vec![WorkerProcessObservation {
        name: "worker-tool".into(),
        arguments: vec![remote_marker],
    }];
    assert!(
        remote_store
            .reserve_next(&remote_nodes(processes))
            .unwrap()
            .is_none(),
        "a worker-observed process must consume that remote node process cap"
    );
}

#[test]
fn incomplete_remote_process_observation_saturates_a_patterned_process_cap() {
    let (root, store) = store_with_process_pattern("incomplete", "unobserved-worker-tool".into());
    let processes = vec![WorkerProcessObservation {
        name: "process-observations-incomplete".into(),
        arguments: Vec::new(),
    }];
    assert!(
        store
            .reserve_next(&remote_nodes(processes))
            .unwrap()
            .is_none(),
        "an incomplete remote observation must not underbook a process cap"
    );
    let attempts = Connection::open(root.path().join("takd.sqlite"))
        .unwrap()
        .query_row("SELECT COUNT(*) FROM run_attempts", [], |row| {
            row.get::<_, i64>(0)
        })
        .unwrap();
    assert_eq!(attempts, 0, "a saturated process cap acquired a lease");
}

fn store_with_process_pattern(key: &str, pattern: String) -> (tempfile::TempDir, RunStore) {
    let root = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(root.path().join("takd.sqlite")).unwrap();
    let mut request = project_process_cap(independent_jobs(key, 1));
    let LimiterDefinition::ProcessCap { match_pattern, .. } =
        &mut request.run.limiter_definitions[0]
    else {
        unreachable!()
    };
    *match_pattern = Some(pattern);
    commit(&store, &request, "alice");
    (root, store)
}

fn remote_nodes(processes: Vec<WorkerProcessObservation>) -> Vec<SchedulerNode> {
    let peers = v2_node_loss::restarted_peer_manager("http://127.0.0.1:9");
    let mut snapshot = v2_node_loss::snapshot("worker-a");
    snapshot.processes = processes;
    peers.mark_worker_snapshot("worker-a", snapshot);
    peers.scheduler_nodes()
}
