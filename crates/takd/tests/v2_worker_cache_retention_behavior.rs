use std::time::{Duration, SystemTime};

use takd::{RemoteRuntimeConfig, SubmitAttemptStore, run_worker_http_server};
use tokio::net::TcpListener;

use crate::support::{
    env::{EnvGuard, env_lock},
    remote_output::test_context_with_runtime,
};

#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "multi_thread")]
async fn worker_cache_gc_runs_at_startup_and_periodically_with_one_lru_budget() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("MOCK_CONTAINER", "1");
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = seed_workspace(root, 'a', b"old!", SystemTime::UNIX_EPOCH);
    let recent = seed_path(root, 'b', b"new!", 2);
    let runtime = RemoteRuntimeConfig::from_environment(
        |name| match name {
            "TAKD_WORKER_CACHE_BUDGET_BYTES" => Some("4".into()),
            "TAKD_REMOTE_CLEANUP_INTERVAL_MS" => Some("20".into()),
            _ => None,
        },
        root.to_path_buf(),
        true,
    );
    let context = test_context_with_runtime(runtime).with_state_root(root);
    let store = SubmitAttemptStore::with_db_path(root.join("agent.sqlite")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server = tokio::spawn(run_worker_http_server(listener, store, context));

    wait_until_missing(&old).await;
    assert!(recent.exists());
    let periodic = seed_workspace(root, 'c', b"next", SystemTime::UNIX_EPOCH);
    wait_until_missing(&periodic).await;
    assert!(recent.exists());

    server.abort();
    let _ = server.await;
}

fn seed_workspace(
    root: &std::path::Path,
    key: char,
    bytes: &[u8],
    modified: SystemTime,
) -> std::path::PathBuf {
    let path = root
        .join("worker-v2-workspace-cache")
        .join(format!("{}.tar", key.to_string().repeat(64)));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, bytes).unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    path
}

fn seed_path(root: &std::path::Path, key: char, bytes: &[u8], accessed: u64) -> std::path::PathBuf {
    let path = root
        .join("worker-v2-path-caches")
        .join(key.to_string().repeat(64));
    std::fs::create_dir_all(&path).unwrap();
    std::fs::write(path.join("value"), bytes).unwrap();
    std::fs::write(path.join(".last-accessed-ms"), accessed.to_string()).unwrap();
    path
}

async fn wait_until_missing(path: &std::path::Path) {
    tokio::time::timeout(Duration::from_secs(10), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap_or_else(|_| {
        panic!(
            "timed out waiting for cache GC to remove {}",
            path.display()
        )
    });
}
