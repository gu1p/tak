#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::{RemoteSpec, RemoteTransportKind};
use tak_exec::{RunOptions, run_tasks};

mod support;

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
    }
}

#[tokio::test]
async fn remote_execution_with_any_transport_falls_through_unreachable_direct_to_reachable_tor() {
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

    let tor_server = RunningTakdServer::spawn("builder-tor", "tor", temp.path()).await;
    env.set("TAK_TEST_TOR_ONION_DIAL_ADDR", tor_server.bind_addr.clone());
    write_remote_inventory(
        &config_root,
        &[
            RemoteInventoryRecord::builder(
                "builder-direct-unreachable",
                "http://127.0.0.1:9",
                "secret",
                "direct",
            ),
            RemoteInventoryRecord::builder(
                &tor_server.node_id,
                &tor_server.base_url,
                &tor_server.bearer_token,
                "tor",
            ),
        ],
    );

    let (spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_any_transport_tor",
        vec![shell_step("mkdir -p dist && echo tor > dist/out.txt")],
        any_transport_remote_spec(),
        vec![workspace_output_path("dist/out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("any-transport remote run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(result.remote_node_id.as_deref(), Some("builder-tor"));
    assert_eq!(result.remote_transport_kind.as_deref(), Some("tor"));
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("tor output"),
        "tor\n"
    );
}
