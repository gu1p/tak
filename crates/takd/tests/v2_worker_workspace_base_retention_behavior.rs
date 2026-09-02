use std::time::Duration;

use takd::{RemoteRuntimeConfig, SubmitAttemptStore, run_worker_http_server};
use tokio::net::TcpListener;

use crate::support::remote_output::test_context_with_runtime;

#[tokio::test]
async fn worker_gc_counts_and_evicts_an_extracted_base_with_its_archive() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let fingerprint = "a".repeat(64);
    let archive = root
        .join("worker-v2-workspace-cache")
        .join(format!("{fingerprint}.tar"));
    std::fs::create_dir_all(archive.parent().unwrap()).unwrap();
    std::fs::write(&archive, b"four").unwrap();
    let base = root.join("worker-v2-workspace-bases").join(&fingerprint);
    std::fs::create_dir_all(base.join("data")).unwrap();
    std::fs::write(base.join("data/value"), b"base").unwrap();
    std::fs::write(base.join("ready"), b"v2\n").unwrap();

    let runtime = RemoteRuntimeConfig::from_environment(
        |name| (name == "TAKD_WORKER_CACHE_BUDGET_BYTES").then(|| "4".into()),
        root.to_path_buf(),
        true,
    );
    let context = test_context_with_runtime(runtime).with_state_root(root);
    let store = SubmitAttemptStore::with_db_path(root.join("agent.sqlite")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = tokio::spawn(run_worker_http_server(listener, store, context));

    wait_until(|| !archive.exists() && !base.exists()).await;
    server.abort();
}

async fn wait_until(predicate: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
