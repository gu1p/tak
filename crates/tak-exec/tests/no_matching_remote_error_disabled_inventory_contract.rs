#![allow(clippy::await_holding_lock)]

use tak_core::model::RemoteTransportKind;
use tak_exec::{NoMatchingRemoteError, RunOptions, run_tasks};

use crate::support;

use support::{
    EnvGuard, env_lock, prepare_workspace, remote_builder_spec, remote_task_spec, shell_step,
    write_disabled_remote,
};

#[tokio::test]
async fn no_matching_remote_error_reports_when_inventory_has_no_enabled_remotes() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let (_temp, workspace_root, config_root) = prepare_workspace(&mut env);
    write_disabled_remote(&config_root);

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_requires_enabled_node",
        vec![shell_step("echo should-not-run")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );
    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("run should fail without an enabled remote");
    let diagnostic = err
        .downcast_ref::<NoMatchingRemoteError>()
        .expect("expected a structured no-matching-remote error");

    assert_eq!(diagnostic.configured_remote_count, 1);
    assert_eq!(diagnostic.enabled_remote_count, 0);
    assert!(diagnostic.enabled_remotes.is_empty());
}
