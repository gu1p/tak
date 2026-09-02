use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use tak_core::v2::Step;
use tak_proto::worker_v2::{WorkerWorkspaceReuse, payload_digest};

use crate::support::{
    env::{EnvGuard, env_lock},
    worker_http::start_server_with_runtime,
    v2_worker_paths,
    v2_worker_shared::{send, wait_terminal},
};

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn worker_gc_never_evicts_a_leased_path_cache() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("MOCK_CONTAINER");
    let runtime = takd::RemoteRuntimeConfig::from_environment(
        |name| match name {
            "TAKD_WORKER_CACHE_BUDGET_BYTES" => Some("1".into()),
            "TAKD_REMOTE_CLEANUP_INTERVAL_MS" => Some("200".into()),
            "TAKD_MEMORY_PRESSURE_ENABLED" => Some("false".into()),
            "TAK_TEST_IGNORE_HOST_USAGE" => Some("true".into()),
            _ => None,
        },
        std::env::temp_dir(),
        true,
    );
    let server = start_server_with_runtime(runtime).await;
    let mut request = v2_worker_paths::consume();
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "sleep 2".into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    request.payload_digest = payload_digest(&request.payload).unwrap();
    let leased = path_root(&server.state_root, &request);
    seed_path_cache(&leased);
    send(&server, &request, &[]).await;
    wait_until(|| std::fs::read_to_string(leased.join(".last-accessed-ms")).unwrap() != "0").await;
    let victim = seed_victim(&server.state_root);

    wait_until(|| !victim.exists()).await;
    assert!(leased.exists());
    wait_terminal(&server, &request).await;
}

fn path_root(
    state_root: &std::path::Path,
    request: &tak_proto::worker_v2::DispatchAttemptRequest,
) -> std::path::PathBuf {
    let WorkerWorkspaceReuse::Paths { session_id, .. } = &request.payload.workspace_reuse else {
        panic!("expected paths reuse");
    };
    let key = serde_json::to_string(&(
        &request.identity.run_id,
        &request.identity.node_id,
        session_id,
    ))
    .unwrap();
    state_root
        .join("worker-v2-path-caches")
        .join(format!("{:x}", Sha256::digest(key)))
}

fn seed_path_cache(root: &std::path::Path) {
    let generation = root.join("generations/1/.cache");
    std::fs::create_dir_all(&generation).unwrap();
    std::fs::write(generation.join("value"), b"warm").unwrap();
    std::fs::write(root.join("current"), b"1\n").unwrap();
    std::fs::write(root.join(".last-accessed-ms"), b"0").unwrap();
}

fn seed_victim(state_root: &std::path::Path) -> std::path::PathBuf {
    let path = state_root
        .join("worker-v2-workspace-cache")
        .join(format!("{}.tar", "d".repeat(64)));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"drop").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(SystemTime::UNIX_EPOCH))
        .unwrap();
    path
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
