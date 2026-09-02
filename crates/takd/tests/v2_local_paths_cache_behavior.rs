use std::time::Duration;

use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

use crate::support::{protocol_server::spawn_protocol_server, v2_run};

#[tokio::test]
async fn paths_session_restores_a_private_snapshot_without_publishing_it_as_output() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db).unwrap();
    let request = v2_run::path_cache::dependent_run("local-path-cache");
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
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, "dist/result");
    server.abort();
}

async fn wait_for(predicate: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(15), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
