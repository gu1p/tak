#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::{
    CurrentStateOrigin, CurrentStateSpec, IgnoreSourceSpec, PathAnchor, PathRef, normalize_path_ref,
};
use tak_exec::{PlacementMode, RunOptions, run_tasks};

use crate::support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec,
    remote_task_spec_with_context_and_outputs, shell_step, workspace_output_path,
    write_remote_inventory,
};

#[tokio::test]
async fn remote_execution_uses_workspace_gitignores_and_readds_included_subtree() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");

    fs::create_dir_all(workspace_root.join("src")).expect("create src");
    fs::create_dir_all(workspace_root.join("dist")).expect("create dist");
    fs::create_dir_all(workspace_root.join("apps/web/generated/reinclude"))
        .expect("create generated");
    fs::write(workspace_root.join(".gitignore"), "dist/\n").expect("write root gitignore");
    fs::create_dir_all(workspace_root.join("apps/web")).expect("create apps/web");
    fs::write(workspace_root.join("apps/web/.gitignore"), "generated/\n")
        .expect("write nested gitignore");
    fs::write(workspace_root.join("src/input.txt"), "visible\n").expect("write input");
    fs::write(workspace_root.join("dist/root.txt"), "hidden\n").expect("write root ignored");
    fs::write(
        workspace_root.join("apps/web/generated/ignored.txt"),
        "hidden nested\n",
    )
    .expect("write nested ignored");
    fs::write(
        workspace_root.join("apps/web/generated/reinclude/keep.txt"),
        "keep me\n",
    )
    .expect("write reinclude");

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
        "remote_gitignore",
        vec![shell_step(
            "test -f src/input.txt && \
             test -f apps/web/generated/reinclude/keep.txt && \
             test ! -e apps/web/generated/ignored.txt && \
             test ! -e dist/root.txt && \
             mkdir -p out && \
             find apps/web/generated -type f | LC_ALL=C sort > out/generated.txt",
        )],
        remote_builder_spec(tak_core::model::RemoteTransportKind::Direct),
        CurrentStateSpec {
            roots: vec![PathRef {
                anchor: PathAnchor::Workspace,
                path: ".".to_string(),
            }],
            ignored: vec![IgnoreSourceSpec::GitIgnore],
            include: vec![
                normalize_path_ref("workspace", "apps/web/generated/reinclude")
                    .expect("include path"),
            ],
            origin: CurrentStateOrigin::Explicit,
        },
        vec![workspace_output_path("out/generated.txt")],
    );

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run should succeed");
    let result = summary.results.get(&label).expect("summary result");
    let generated_path = workspace_root.join("out/generated.txt");
    let generated = fs::read_to_string(&generated_path).expect("synced output");

    assert_eq!(result.placement_mode, PlacementMode::Remote);
    assert_eq!(generated, "apps/web/generated/reinclude/keep.txt\n");
}
