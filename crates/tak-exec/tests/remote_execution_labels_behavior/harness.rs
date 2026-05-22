use std::collections::BTreeMap;

use tak_core::model::{TaskLabel, WorkspaceSpec};
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    EnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord, env_lock,
    write_remote_inventory,
};

pub(super) async fn run_and_collect_labels(
    build_workspace: impl FnOnce(&std::path::Path) -> (WorkspaceSpec, Vec<TaskLabel>),
) -> BTreeMap<String, Option<String>> {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());

    let events = RecordingEvents::default();
    let server = RecordingRemoteServer::spawn_success("builder-a", events.clone());
    write_remote_inventory(
        &config,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            "secret",
            "direct",
        )],
    );

    let (spec, targets) = build_workspace(&workspace);
    run_tasks(&spec, &targets, &RunOptions::default())
        .await
        .expect("remote run");

    events
        .submit_payloads()
        .into_iter()
        .map(|payload| (payload.task_label, payload.execution_label))
        .collect()
}
