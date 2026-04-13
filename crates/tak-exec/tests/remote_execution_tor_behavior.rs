#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

mod support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec,
    remote_task_spec_with_outputs, shell_step, workspace_output_path, write_remote_inventory,
};

#[tokio::test]
async fn simulated_tor_remote_execution_uses_test_onion_dial_addr() {
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

    let server = RunningTakdServer::spawn("builder-tor", "tor", temp.path()).await;
    env.set("TAK_TEST_TOR_ONION_DIAL_ADDR", server.bind_addr.clone());
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            &server.bearer_token,
            "tor",
        )],
    );

    let (spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_tor",
        vec![shell_step("mkdir -p dist && echo tor > dist/out.txt")],
        remote_builder_spec(RemoteTransportKind::Tor),
        vec![workspace_output_path("dist/out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("tor remote run should succeed");

    assert_eq!(
        summary
            .results
            .get(&label)
            .and_then(|result| result.remote_node_id.as_deref()),
        Some("builder-tor")
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("tor output"),
        "tor\n"
    );
}
