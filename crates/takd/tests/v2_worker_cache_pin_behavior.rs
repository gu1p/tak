use std::collections::BTreeMap;
use std::time::Duration;

use tak_core::v2::Step;
use tak_proto::worker_v2::payload_digest;

use crate::support::{
    env::{EnvGuard, env_lock},
    worker_http::start_server_with_runtime,
    v2_worker_cache::upload,
    v2_worker_shared::{dispatch_with_seed, seed_archive, send, wait_terminal},
};

#[allow(clippy::await_holding_lock)]
#[tokio::test]
async fn worker_gc_preserves_pinned_shared_and_in_transfer_data() {
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
    let mut pinned = dispatch_with_seed(1, "fence-pinned", "pin", true);
    pinned.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "sleep 2".into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    pinned.payload_digest = payload_digest(&pinned.payload).unwrap();
    let archive = seed_archive("pin");
    send(&server, &pinned, &archive).await;

    let victim = dispatch_with_seed(2, "fence-victim", "old", true);
    upload(
        &server,
        &victim.payload.workspace.descriptor,
        &seed_archive("old"),
    )
    .await;
    let cache = server.state_root.join("worker-v2-workspace-cache");
    let pinned_blob = blob(&cache, &pinned);
    let victim_blob = blob(&cache, &victim);
    let transfer = cache.join(".transfer-protected");
    std::fs::write(&transfer, b"partial").unwrap();

    wait_until_missing(&victim_blob).await;
    assert!(pinned_blob.exists());
    assert!(transfer.exists());
    assert!(has_child(&server.state_root.join("worker-v2-shared")));
    wait_terminal(&server, &pinned).await;
}

fn blob(
    root: &std::path::Path,
    request: &tak_proto::worker_v2::DispatchAttemptRequest,
) -> std::path::PathBuf {
    root.join(format!(
        "{}.tar",
        request.payload.workspace.descriptor.manifest.fingerprint
    ))
}

fn has_child(root: &std::path::Path) -> bool {
    std::fs::read_dir(root)
        .map(|mut children| children.next().is_some())
        .unwrap_or(false)
}

async fn wait_until_missing(path: &std::path::Path) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while path.exists() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
