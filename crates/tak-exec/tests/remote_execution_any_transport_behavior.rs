#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::{RemoteSelectionSpec, RemoteSpec, RemoteTransportKind};
use tak_exec::{PlacementMode, RunOptions, run_tasks};

use crate::support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_task_spec_with_outputs,
    shell_step, workspace_output_path, write_remote_inventory,
};

fn any_transport_remote_spec() -> RemoteSpec {
    RemoteSpec {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()],
        transport_kind: RemoteTransportKind::Any,
        runtime: None,
        selection: RemoteSelectionSpec::Sequential,
    }
}

#[tokio::test]
async fn remote_execution_with_any_transport_uses_direct_remote_and_reports_direct_transport() {
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

    let server = RunningTakdServer::spawn("builder-direct", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            &server.bearer_token,
            "direct",
        )],
    );

    let (spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_any_transport_direct",
        vec![shell_step("mkdir -p dist && echo direct > dist/out.txt")],
        any_transport_remote_spec(),
        vec![workspace_output_path("dist/out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("any-transport remote run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(result.placement_mode, PlacementMode::Remote);
    assert_eq!(result.remote_node_id.as_deref(), Some("builder-direct"));
    assert_eq!(result.remote_transport_kind.as_deref(), Some("direct"));
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("direct output"),
        "direct\n"
    );
}
