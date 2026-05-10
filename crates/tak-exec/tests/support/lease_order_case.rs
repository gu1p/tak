use std::fs;
use std::sync::MutexGuard;

use tak_core::model::{TaskLabel, WorkspaceSpec};
use tak_exec::RunOptions;

use super::{
    EnvGuard, RecordingEvents, RecordingLeaseServer, RecordingRemoteServer, RemoteInventoryRecord,
    add_ui_lock_need, env_lock, remote_builder_spec, remote_task_spec, shell_step,
    write_remote_inventory,
};

pub struct RemoteLeaseCase {
    pub _env_lock: MutexGuard<'static, ()>,
    pub _env: EnvGuard,
    pub _temp: tempfile::TempDir,
    pub events: RecordingEvents,
    pub _lease: RecordingLeaseServer,
    pub _remote: RecordingRemoteServer,
    pub spec: WorkspaceSpec,
    pub label: TaskLabel,
    pub options: RunOptions,
}

pub async fn remote_lease_case(name: &str) -> RemoteLeaseCase {
    remote_lease_case_with_remote(name, RecordingRemoteServer::spawn_success).await
}

pub async fn remote_lease_case_with_submit_failure(name: &str) -> RemoteLeaseCase {
    remote_lease_case_with_remote(name, RecordingRemoteServer::spawn_submit_failure).await
}

async fn remote_lease_case_with_remote(
    name: &str,
    spawn_remote: fn(&str, RecordingEvents) -> RecordingRemoteServer,
) -> RemoteLeaseCase {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");

    let events = RecordingEvents::default();
    let lease = RecordingLeaseServer::spawn(&temp.path().join("takd.sock"), events.clone()).await;
    let remote = spawn_remote(&format!("builder-{name}"), events.clone());
    let env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            &remote.node_id,
            &remote.base_url,
            "secret",
            "direct",
        )],
    );

    let (mut spec, label) = remote_task_spec(
        &workspace_root,
        "remote_with_needs",
        vec![shell_step("true")],
        remote_builder_spec(tak_core::model::RemoteTransportKind::Direct),
    );
    add_ui_lock_need(&mut spec, &label);
    let options = RunOptions {
        lease_socket: Some(lease.socket_path.clone()),
        ..RunOptions::default()
    };
    RemoteLeaseCase {
        _env_lock: env_lock,
        _env: env,
        _temp: temp,
        events,
        _lease: lease,
        _remote: remote,
        spec,
        label,
        options,
    }
}
