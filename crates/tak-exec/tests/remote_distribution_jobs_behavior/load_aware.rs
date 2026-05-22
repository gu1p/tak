use super::support::{node_status, remote_workspace};
use crate::support::{
    EnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord, env_lock,
    write_remote_inventory,
};
use tak_core::model::TaskLabel;
use tak_exec::{RunOptions, run_tasks};

#[tokio::test]
async fn shuffled_remote_jobs_prefer_less_loaded_reachable_node() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    let events = RecordingEvents::default();
    let busy = RecordingRemoteServer::spawn_success_with_status(
        "builder-busy",
        events.clone(),
        node_status("builder-busy", 1, 0),
    );
    let idle = RecordingRemoteServer::spawn_success_with_status(
        "builder-idle",
        events,
        node_status("builder-idle", 0, 0),
    );
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder(&busy.node_id, &busy.base_url, "secret", "direct"),
            RemoteInventoryRecord::builder(&idle.node_id, &idle.base_url, "secret", "direct"),
        ],
    );

    let label = TaskLabel {
        package: "//".into(),
        name: "check".into(),
    };
    let spec = remote_workspace(&workspace, std::slice::from_ref(&label));
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run");

    assert_eq!(
        summary
            .results
            .get(&label)
            .and_then(|result| result.remote_node_id.as_deref()),
        Some("builder-idle")
    );
}

#[tokio::test]
async fn shuffled_remote_jobs_prefer_known_status_over_unknown_status() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    let events = RecordingEvents::default();
    let unknown = RecordingRemoteServer::spawn_success("builder-unknown", events.clone());
    let known = RecordingRemoteServer::spawn_success_with_status(
        "builder-known",
        events,
        node_status("builder-known", 1, 0),
    );
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder(&unknown.node_id, &unknown.base_url, "secret", "direct"),
            RemoteInventoryRecord::builder(&known.node_id, &known.base_url, "secret", "direct"),
        ],
    );

    let label = TaskLabel {
        package: "//".into(),
        name: "check".into(),
    };
    let spec = remote_workspace(&workspace, std::slice::from_ref(&label));
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run");

    assert_eq!(
        summary
            .results
            .get(&label)
            .and_then(|result| result.remote_node_id.as_deref()),
        Some("builder-known")
    );
}
