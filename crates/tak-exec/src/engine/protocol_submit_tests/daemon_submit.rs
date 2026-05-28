use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::{Mutex, MutexGuard};

use super::super::protocol_result_http::remote_protocol_http_request_with_extra_headers;

#[path = "daemon_submit/support.rs"]
mod support;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

async fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().await
}

#[tokio::test]
async fn tor_daemon_submit_sends_placement_requirements_not_client_node_pin() {
    let _env_lock = env_lock().await;
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("takd.sock");
    unsafe { std::env::set_var("TAKD_SOCKET", &socket_path) };
    let daemon = support::spawn_submit_daemon(&socket_path).await;

    let response = remote_protocol_http_request_with_extra_headers(
        &support::tor_target(),
        "POST",
        "/v1/tasks/submit",
        Some(&support::submit_request_body()),
        "submit",
        Duration::from_secs(1),
        &[],
    )
    .await
    .expect("daemon submit response");

    let request = daemon.await.expect("daemon task");
    assert!(request.contains(r#""pool":"build""#));
    assert!(request.contains(r#""tags":["linux"]"#));
    assert!(request.contains(r#""capabilities":["docker"]"#));
    assert!(!request.contains("node:builder-client-choice"));
    assert_eq!(
        response.daemon_task_handle.as_deref(),
        Some("daemon-task-7")
    );
    assert_eq!(
        response.daemon_peer_node_id.as_deref(),
        Some("builder-daemon-choice")
    );
    unsafe { std::env::remove_var("TAKD_SOCKET") };
}
