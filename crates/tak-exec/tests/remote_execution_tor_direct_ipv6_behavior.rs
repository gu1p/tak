#![allow(clippy::await_holding_lock)]

mod support;

use std::fs;
use std::io::ErrorKind;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};
use tak_proto::NodeInfo;
use takd::daemon::remote::{RemoteNodeContext, SubmitAttemptStore, run_remote_v1_http_server};
use tokio::net::TcpListener;

use support::{
    EnvGuard, RemoteInventoryRecord, env_lock, remote_builder_spec, remote_task_spec_with_outputs,
    shell_step, workspace_output_path, write_remote_inventory,
};

#[tokio::test]
async fn tor_transport_reaches_non_onion_ipv6_remotes() {
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

    let listener = match TcpListener::bind("[::1]:0").await {
        Ok(listener) => listener,
        Err(err) if err.kind() == ErrorKind::AddrNotAvailable => return,
        Err(err) => panic!("bind ipv6 remote listener: {err}"),
    };
    let bind_addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{bind_addr}");
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-ipv6".into(),
            display_name: "builder-ipv6".into(),
            base_url: base_url.clone(),
            healthy: true,
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("builder-ipv6.sqlite"))
        .expect("submit attempt store");
    let server = tokio::spawn(async move {
        let _ = run_remote_v1_http_server(listener, store, context).await;
    });
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-ipv6",
            &base_url,
            "secret",
            "tor",
        )],
    );

    let (spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_tor_ipv6_direct",
        vec![shell_step("mkdir -p dist && echo ipv6 > dist/out.txt")],
        remote_builder_spec(RemoteTransportKind::Tor),
        vec![workspace_output_path("dist/out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("tor transport should reach non-onion ipv6 remotes");

    assert_eq!(
        summary
            .results
            .get(&label)
            .and_then(|result| result.remote_node_id.as_deref()),
        Some("builder-ipv6")
    );
    assert_eq!(
        summary
            .results
            .get(&label)
            .and_then(|result| result.remote_transport_kind.as_deref()),
        Some("tor")
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("ipv6 output"),
        "ipv6\n"
    );
    server.abort();
}
