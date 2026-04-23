use std::fs;
use std::path::Path;

use takd::{SubmitAttemptStore, build_submit_idempotency_key, run_remote_v1_http_server};
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};

use crate::support;

use takd::RemoteRuntimeConfig;

use support::env::{EnvGuard, env_lock};
use support::remote_output::{submit_shell_task, test_context_with_runtime};

#[test]
fn cleanup_janitor_removes_stale_roots_but_preserves_active_jobs() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TEST_HOST_PLATFORM", "other");
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let artifact_root = temp.path().join("takd-remote-artifacts");
    fs::create_dir_all(&exec_root).expect("create exec root");
    fs::create_dir_all(&artifact_root).expect("create artifact root");

    let stale_exec_root = exec_root.join("stale-job_1");
    let stale_artifact_root = artifact_root.join("stale-job_1");
    fs::create_dir_all(&stale_exec_root).expect("create stale exec root");
    fs::create_dir_all(&stale_artifact_root).expect("create stale artifact root");

    Runtime::new().expect("runtime").block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind remote listener");
        let context = test_context_with_runtime(
            RemoteRuntimeConfig::for_tests()
                .with_explicit_remote_exec_root(exec_root.clone())
                .with_skip_exec_root_probe(true)
                .with_remote_cleanup_ttl(Duration::from_millis(10))
                .with_remote_cleanup_interval(Duration::from_millis(10)),
        );
        let store =
            SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
        let server = tokio::spawn(run_remote_v1_http_server(
            listener,
            store.clone(),
            context.clone(),
        ));

        let submit_ack = submit_shell_task(&context, &store, "active-job", "sleep 5");
        assert!(submit_ack.accepted);

        let active_key = build_submit_idempotency_key("active-job", Some(1)).expect("active key");
        let active_root_name = active_key.replace(':', "_");
        let active_exec_root = exec_root.join(&active_root_name);
        let active_artifact_root = artifact_root.join(&active_root_name);
        fs::create_dir_all(&active_artifact_root).expect("create active artifact root");

        wait_for_presence(&active_exec_root).await;
        wait_for_removal(&stale_exec_root).await;
        wait_for_removal(&stale_artifact_root).await;

        assert!(
            active_exec_root.exists(),
            "active execution root should be preserved"
        );
        assert!(
            active_artifact_root.exists(),
            "active artifact root should be preserved"
        );

        server.abort();
        let _ = server.await;
    });
}

async fn wait_for_removal(path: &Path) {
    for _ in 0..50 {
        if !path.exists() {
            return;
        }
        sleep(Duration::from_millis(20)).await;
    }

    panic!("timed out waiting for cleanup of {}", path.display());
}

async fn wait_for_presence(path: &Path) {
    for _ in 0..100 {
        if path.exists() {
            return;
        }
        sleep(Duration::from_millis(20)).await;
    }

    panic!("timed out waiting for creation of {}", path.display());
}
