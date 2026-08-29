#![allow(clippy::await_holding_lock)]

use std::{fs, time::Duration};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    pressure::wait_for_task_creates,
    remote_container::configure_fake_docker_env,
    remote_output::test_context_with_runtime,
    remote_session::{cancel_session_task, session, submit_session_task},
    sqlite_gate::begin_immediate,
    wait_for_path::{backdate, wait_for_path, wait_for_path_refreshed},
    wait_for_session_task::{assert_cancelled_result, wait_for_session_task_inactive},
    wait_for_terminal_events::wait_for_terminal_events,
};
use takd::{SubmitAttemptStore, run_remote_v1_http_server};

#[tokio::test(flavor = "multi_thread")]
async fn cleanup_janitor_preserves_an_active_shared_workspace() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let exec_root = temp.path().join("exec-root");
    let crash_tombstone = exec_root.join(".tak-cleanup@crashed-session");
    fs::create_dir_all(&crash_tombstone).expect("create crash tombstone");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind remote listener");
    let daemon = FakeDockerDaemon::spawn(
        temp.path(),
        FakeDockerConfig {
            visible_roots: vec![exec_root.clone()],
            wait_response_delay: Duration::from_secs(600),
            ..Default::default()
        },
    );
    let runtime = configure_fake_docker_env(temp.path(), daemon.socket_path(), &mut env)
        .with_explicit_remote_exec_root(exec_root.clone())
        .with_skip_exec_root_probe(true)
        .with_remote_cleanup_ttl(Duration::from_secs(600))
        .with_remote_cleanup_interval(Duration::from_millis(10))
        .build();
    let context = test_context_with_runtime(runtime);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));
    wait_for_path(&crash_tombstone, false, "crash tombstone reaping").await;

    submit_session_task(
        &context,
        &store,
        "active-session-task",
        "true",
        session("active-session", "share_workspace", Vec::new()),
    );
    let session_root = exec_root.join("sessions/active-session");
    wait_for_path(&session_root, true, "creation").await;
    wait_for_task_creates(&daemon, "active-session-task", 1);
    let sentinel = session_root.join("still-active");
    fs::write(&sentinel, b"present").expect("write sentinel");
    let first_baseline = backdate(session_root.parent().expect("session storage parent"));
    let stale_root = exec_root.join("stale-job");
    fs::create_dir(&stale_root).expect("create stale root");
    backdate(&stale_root);
    wait_for_path(&stale_root, false, "cleanup").await;

    assert!(
        sentinel.exists(),
        "janitor deleted an active shared workspace"
    );
    let db_gate = begin_immediate(&temp.path().join("agent.sqlite"));
    assert!(cancel_session_task(&context, &store, "active-session-task"));
    let session_parent = session_root.parent().expect("session storage parent");
    wait_for_path_refreshed(session_parent, first_baseline).await;
    let final_baseline = backdate(session_parent);
    db_gate.execute_batch("COMMIT").expect("unlock store");
    wait_for_path_refreshed(session_parent, final_baseline).await;
    wait_for_terminal_events(&context, &store, "active-session-task");
    assert_cancelled_result(&store, "active-session-task");
    wait_for_session_task_inactive(&context, &store, "active-session-task").await;
    let post_terminal_probe = temp.path().join("takd-remote-artifacts/session-paths");
    fs::create_dir_all(&post_terminal_probe).expect("create post-terminal probe");
    backdate(&post_terminal_probe);
    wait_for_path(&post_terminal_probe, false, "post-terminal cleanup").await;
    assert!(
        sentinel.exists(),
        "janitor immediately deleted a just-completed shared workspace"
    );
    server.abort();
    let _ = server.await;
}
