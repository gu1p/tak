#![allow(clippy::await_holding_lock)]

use std::fs;
use std::net::TcpListener as StdTcpListener;
use std::time::Duration;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};
use tak_proto::NodeInfo;
use takd::daemon::remote::{RemoteNodeContext, SubmitAttemptStore, run_remote_v1_http_server};
use tokio::net::TcpListener;

mod support;

use support::{
    EnvGuard, RemoteInventoryRecord, env_lock, remote_builder_spec, remote_task_spec_with_outputs,
    shell_step, workspace_output_path, write_remote_inventory,
};

#[tokio::test]
async fn simulated_tor_remote_execution_retries_until_the_hidden_service_listener_appears() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    let bind_addr = {
        let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind free port");
        let addr = listener.local_addr().expect("free addr").to_string();
        drop(listener);
        addr
    };
    env.set("TAK_TEST_TOR_ONION_DIAL_ADDR", bind_addr.clone());
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-tor-delayed",
            "http://builder-tor-delayed.onion",
            "secret",
            "tor",
        )],
    );
    let delayed_server = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let listener = TcpListener::bind(&bind_addr)
            .await
            .expect("bind delayed listener");
        let context = RemoteNodeContext::new(
            NodeInfo {
                node_id: "builder-tor-delayed".into(),
                display_name: "builder-tor-delayed".into(),
                base_url: "http://builder-tor-delayed.onion".into(),
                healthy: true,
                pools: vec!["build".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: "tor".into(),
            },
            "secret".into(),
        );
        let store = SubmitAttemptStore::with_db_path(temp.path().join("delayed.sqlite"))
            .expect("submit store");
        let _ = run_remote_v1_http_server(listener, store, context).await;
    });
    let (spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_tor_delayed",
        vec![shell_step(
            "mkdir -p dist && echo tor-delayed > dist/out.txt",
        )],
        remote_builder_spec(RemoteTransportKind::Tor),
        vec![workspace_output_path("dist/out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("tor remote run should retry until the listener is ready");
    assert_eq!(
        summary
            .results
            .get(&label)
            .and_then(|result| result.remote_node_id.as_deref()),
        Some("builder-tor-delayed")
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("tor output"),
        "tor-delayed\n"
    );
    delayed_server.abort();
}
