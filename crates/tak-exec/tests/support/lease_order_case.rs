use std::fs;
use std::sync::MutexGuard;
use std::time::Duration;

use super::{
    EnvGuard, RecordingEvents, RecordingLeaseConfig, RecordingLeaseServer, RecordingRemoteServer,
    RemoteInventoryRecord, add_ui_lock_need, env_lock, remote_builder_spec, remote_task_spec,
    shell_step, write_remote_inventory,
};
use tak_core::model::{TaskLabel, WorkspaceSpec};
use tak_exec::RunOptions;
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
pub async fn remote_lease_case_with_slow_result(
    name: &str,
    lease_config: RecordingLeaseConfig,
    result_delay: Duration,
) -> RemoteLeaseCase {
    remote_lease_case_with_custom(
        name,
        |node_id, events| {
            RecordingRemoteServer::spawn_success_with_result_delay(node_id, events, result_delay)
        },
        lease_config,
    )
    .await
}
async fn remote_lease_case_with_remote(
    name: &str,
    spawn_remote: fn(&str, RecordingEvents) -> RecordingRemoteServer,
) -> RemoteLeaseCase {
    remote_lease_case_with_custom(name, spawn_remote, RecordingLeaseConfig::default()).await
}
async fn remote_lease_case_with_custom(
    name: &str,
    spawn_remote: impl FnOnce(&str, RecordingEvents) -> RecordingRemoteServer,
    lease_config: RecordingLeaseConfig,
) -> RemoteLeaseCase {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let events = RecordingEvents::default();
    let lease = RecordingLeaseServer::spawn_with_config(
        &temp.path().join("takd.sock"),
        events.clone(),
        lease_config,
    )
    .await;
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
