#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::{
    CurrentStateOrigin, CurrentStateSpec, PathAnchor, PathRef, RemoteTransportKind,
};
use tak_exec::{PlacementMode, RunOptions, run_tasks};

mod support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec,
    remote_task_spec_with_context_and_outputs, shell_step, workspace_output_path,
    write_remote_inventory,
};

#[tokio::test]
async fn explicit_context_without_gitignore_keeps_gitignored_files() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(workspace_root.join("target")).expect("create target");
    fs::write(workspace_root.join(".gitignore"), "target/\n").expect("write gitignore");
    fs::write(workspace_root.join("target/ignored.txt"), "kept\n").expect("write ignored");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );

    let server = RunningTakdServer::spawn("builder-a", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            &server.bearer_token,
            "direct",
        )],
    );

    let (spec, label) = remote_task_spec_with_context_and_outputs(
        &workspace_root,
        "explicit_context",
        vec![shell_step(
            "test -f target/ignored.txt && cp target/ignored.txt out.txt",
        )],
        remote_builder_spec(RemoteTransportKind::Direct),
        CurrentStateSpec {
            roots: vec![PathRef {
                anchor: PathAnchor::Workspace,
                path: ".".to_string(),
            }],
            ignored: Vec::new(),
            include: Vec::new(),
            origin: CurrentStateOrigin::Explicit,
        },
        vec![workspace_output_path("out.txt")],
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run should succeed");

    assert_eq!(
        summary
            .results
            .get(&label)
            .expect("summary result")
            .placement_mode,
        PlacementMode::Remote
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("out.txt")).expect("synced output"),
        "kept\n"
    );
}
