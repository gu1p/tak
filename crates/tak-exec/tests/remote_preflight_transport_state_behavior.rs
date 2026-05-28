#![allow(clippy::await_holding_lock)]

use crate::support;

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RemotePreflightExhaustedError, RemotePreflightFailureKind, RunOptions, run_tasks};
use tak_proto::NodeInfo;
use takd::daemon::remote::{
    RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server,
};
use tokio::net::TcpListener;

use support::{
    EnvGuard, RemoteInventoryRecord, env_lock, remote_builder_spec, remote_task_spec, shell_step,
    write_remote_inventory,
};

#[tokio::test]
async fn remote_preflight_reports_live_recovering_state_before_submit() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("workspace");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let bind_addr = listener.local_addr().expect("listener addr");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-a",
            &format!("http://{bind_addr}"),
            "secret",
            "direct",
        )],
    );
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: format!("http://{bind_addr}"),
            healthy: false,
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "recovering".into(),
            transport_detail: "self-probe failed".into(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let server = tokio::spawn(async move {
        let _ = run_remote_v1_http_server(listener, store, context).await;
    });
    let (spec, label) = remote_task_spec(
        &workspace_root,
        "check",
        vec![shell_step("echo nope")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("preflight should stop on recovering node");
    let error = err
        .downcast_ref::<RemotePreflightExhaustedError>()
        .expect("typed preflight error");
    assert_eq!(
        error.failures[0].kind,
        RemotePreflightFailureKind::Unhealthy
    );
    assert_eq!(
        error.failures[0].live_transport_state.as_deref(),
        Some("recovering")
    );
    assert_eq!(
        error.failures[0].live_transport_detail.as_deref(),
        Some("self-probe failed")
    );
    server.abort();
}
