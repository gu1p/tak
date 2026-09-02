#![allow(clippy::await_holding_lock)]

use tak_proto::local_daemon::v2::{Operation, Request, Response};
use takd::serve_agent;

use crate::support;
use support::env::{EnvGuard, env_lock};
use support::protocol::send_request;

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_without_agent_config_starts_local_daemon_socket() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let root = super::super::relative_temp_root(&temp);
    env.set(
        "XDG_RUNTIME_DIR",
        root.join("runtime").display().to_string(),
    );
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        root.join("remote-exec").display().to_string(),
    );
    let config_root = root.join("config");
    let state_root = root.join("state");
    let socket_path = takd::default_socket_path();

    let config_for_task = config_root.clone();
    let state_for_task = state_root.clone();
    let server = tokio::spawn(async move { serve_agent(&config_for_task, &state_for_task).await });

    let status = wait_for_status(&socket_path).await;
    assert!(
        matches!(status, Response::DaemonStatus { .. }),
        "expected local daemon status without agent config, got {status:?}"
    );

    server.abort();
}

async fn wait_for_status(socket_path: &std::path::Path) -> Response {
    for _ in 0..100 {
        if socket_path.exists() {
            return send_request(
                socket_path,
                &Request {
                    request_id: "daemon-status".into(),
                    operation: Operation::GetDaemonStatus {},
                },
            )
            .await;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!(
        "timed out waiting for daemon socket {}",
        socket_path.display()
    );
}
