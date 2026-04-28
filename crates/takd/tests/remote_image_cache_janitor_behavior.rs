#![allow(clippy::await_holding_lock)]

use std::time::Duration;

use takd::{RemoteImageCacheRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server};
use tokio::{net::TcpListener, time::sleep};

use crate::support;
use support::env::{EnvGuard, env_lock};
use support::fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon};
use support::remote_container::configure_fake_docker_env;
use support::remote_output::test_context_with_runtime;

#[tokio::test(flavor = "multi_thread")]
async fn image_cache_janitor_waits_for_configured_sweep_interval() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let daemon = FakeDockerDaemon::spawn(temp.path(), FakeDockerConfig::default());
    let runtime_config = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_remote_cleanup_interval(Duration::from_millis(10));
    let db_path = temp.path().join("agent.sqlite");
    let context = test_context_with_runtime(runtime_config).with_image_cache_config(
        RemoteImageCacheRuntimeConfig {
            db_path: db_path.clone(),
            budget_bytes: 1,
            mutable_tag_ttl_secs: 86_400,
            sweep_interval_secs: 1,
            low_disk_min_free_percent: 0.0,
            low_disk_min_free_bytes: 0,
        },
    );
    seed_cache_entry(&db_path);

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind remote listener");
    let store =
        SubmitAttemptStore::with_db_path(temp.path().join("agent-store.sqlite")).expect("store");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));

    sleep(Duration::from_millis(250)).await;
    let early = tak_runner::image_cache_status(&db_path, 1, 0.0, 0).expect("early status");
    assert_eq!(early.entry_count, 1);

    wait_for_cache_to_empty(&db_path).await;

    server.abort();
    let _ = server.await;
}

fn seed_cache_entry(db_path: &std::path::Path) {
    let _ = tak_runner::image_cache_status(db_path, 1, 0.0, 0).expect("initialize cache db");
    let conn = rusqlite::Connection::open(db_path).expect("open cache db");
    conn.execute(
        "
        INSERT INTO image_cache_entries (
            cache_key, source_kind, image_ref, image_id, size_bytes,
            created_at_ms, last_used_at_ms, last_refreshed_at_ms
        ) VALUES ('image:missing', 'mutable', 'missing:latest',
                  'sha256:test-image', 1024, 1, 1, 1)
        ",
        [],
    )
    .expect("seed cache entry");
}

async fn wait_for_cache_to_empty(db_path: &std::path::Path) {
    for _ in 0..80 {
        let status = tak_runner::image_cache_status(db_path, 1, 0.0, 0).expect("cache status");
        if status.entry_count == 0 {
            return;
        }
        sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for image cache janitor");
}
