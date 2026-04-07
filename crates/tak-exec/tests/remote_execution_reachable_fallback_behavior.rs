#![allow(clippy::await_holding_lock)]

use std::fs;
use std::net::TcpListener;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

mod support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn remote_execution_falls_back_to_next_reachable_candidate() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );

    let unavailable = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind free port");
        let addr = listener.local_addr().expect("free addr");
        drop(listener);
        format!("http://{addr}")
    };
    let server = RunningTakdServer::spawn("builder-b", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[
            RemoteInventoryRecord::builder("builder-unreachable", &unavailable, "secret", "direct"),
            RemoteInventoryRecord::builder(
                &server.node_id,
                &server.base_url,
                &server.bearer_token,
                "direct",
            ),
        ],
    );

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_fallback",
        vec![shell_step("mkdir -p dist && echo fallback > dist/out.txt")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("fallback run should succeed");

    assert_eq!(
        summary
            .results
            .get(&label)
            .and_then(|result| result.remote_node_id.as_deref()),
        Some("builder-b")
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("fallback output"),
        "fallback\n"
    );
}
