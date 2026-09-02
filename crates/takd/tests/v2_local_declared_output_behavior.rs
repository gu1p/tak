use std::time::Duration;

use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

use crate::support::{protocol_server::spawn_protocol_server, v2_run};

#[tokio::test]
async fn local_declared_outputs_overlay_dependencies_and_are_downloadable() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = temp.path().join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db).unwrap();
    let request = v2_run::local_outputs::dependent_run("local-outputs");
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    wait_for(|| {
        store
            .summary(&run_id)
            .unwrap()
            .is_some_and(|run| run.state.is_terminal())
    })
    .await;

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Succeeded
    );
    let artifacts = store.output_manifest(&run_id).unwrap().unwrap();
    assert_eq!(
        artifacts
            .iter()
            .map(|artifact| artifact.path.as_str())
            .collect::<Vec<_>>(),
        ["dist/result.txt", "generated/input.txt"]
    );
    let result = artifacts
        .iter()
        .find(|artifact| artifact.path == "dist/result.txt")
        .unwrap();
    let chunk = store.output_chunk(&result.artifact_id, 0, 1024).unwrap().unwrap();
    assert_eq!(chunk.bytes, b"producer-consumed");
    assert!(chunk.complete);
    server.abort();
}

async fn wait_for(predicate: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
