#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

mod support;

use support::{
    AuthRejectingSubmitServer, EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock,
    remote_builder_spec, remote_task_spec_with_outputs, shell_step, workspace_output_path,
    write_remote_inventory,
};

#[tokio::test]
async fn remote_execution_retries_submit_on_auth_failure_with_next_candidate() {
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

    let auth_fail = AuthRejectingSubmitServer::spawn("builder-auth-fail");
    let server = RunningTakdServer::spawn("builder-c", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[
            RemoteInventoryRecord::builder(
                "builder-auth-fail",
                &auth_fail.base_url,
                "secret",
                "direct",
            ),
            RemoteInventoryRecord::builder(
                &server.node_id,
                &server.base_url,
                &server.bearer_token,
                "direct",
            ),
        ],
    );

    let (spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_auth_fallback",
        vec![shell_step(
            "mkdir -p dist && echo auth-fallback > dist/out.txt",
        )],
        remote_builder_spec(RemoteTransportKind::Direct),
        vec![workspace_output_path("dist/out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("auth fallback should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(result.remote_node_id.as_deref(), Some("builder-c"));
    assert_eq!(auth_fail.submit_requests(), 1);
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("auth fallback output"),
        "auth-fallback\n"
    );
}
