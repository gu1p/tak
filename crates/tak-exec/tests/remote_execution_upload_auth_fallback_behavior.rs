#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support;
use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, UploadBeginAuthRejectingServer, env_lock,
    remote_builder_spec, remote_task_spec_with_outputs, shell_step, workspace_output_path,
    write_remote_inventory,
};

#[tokio::test]
async fn remote_execution_retries_when_upload_begin_rejects_auth() {
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

    let auth_fail = UploadBeginAuthRejectingServer::spawn("builder-upload-auth-fail");
    let server = RunningTakdServer::spawn("builder-c", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[
            RemoteInventoryRecord::builder(
                "builder-upload-auth-fail",
                &auth_fail.base_url,
                "expired",
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
        "remote_upload_auth_fallback",
        vec![shell_step(
            "mkdir -p dist && echo upload-auth-fallback > dist/out.txt",
        )],
        remote_builder_spec(RemoteTransportKind::Direct),
        vec![workspace_output_path("dist/out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("upload auth fallback should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(result.remote_node_id.as_deref(), Some("builder-c"));
    assert_eq!(auth_fail.begin_requests(), 1);
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("fallback output"),
        "upload-auth-fallback\n"
    );
}
