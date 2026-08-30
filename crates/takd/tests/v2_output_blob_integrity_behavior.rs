use std::time::Duration;

use takd::RunStore;

use crate::support::{protocol_server::spawn_protocol_server, v2_run};

#[tokio::test]
async fn same_size_output_cas_corruption_is_rejected_before_download() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = v2_run::local_outputs::dependent_run("output-corruption");
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    wait_for(|| store.summary(&run_id).unwrap().unwrap().state.is_terminal()).await;
    let artifact = store
        .output_manifest(&run_id)
        .unwrap()
        .unwrap()
        .into_iter()
        .find(|artifact| artifact.path == "dist/result.txt")
        .unwrap();
    let blob = db
        .with_extension("v2-blobs")
        .join("outputs")
        .join(&artifact.sha256);
    std::fs::write(blob, vec![b'x'; artifact.size as usize]).unwrap();

    let error = store.output_chunk(&artifact.artifact_id, 0, 1024).unwrap_err();

    assert!(error.to_string().contains("digest"), "{error:#}");
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
