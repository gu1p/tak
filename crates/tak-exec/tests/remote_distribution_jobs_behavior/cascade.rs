use crate::support::{
    EnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord, env_lock,
    write_remote_inventory,
};
use tak_exec::{RunOptions, run_tasks};

#[path = "cascade/workspace.rs"]
mod workspace;

use workspace::{cascade_workspace, result_node, root_label};

#[tokio::test]
async fn shuffled_cascade_members_select_remote_nodes_independently() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());

    let events = RecordingEvents::default();
    let first = RecordingRemoteServer::spawn_success("builder-a", events.clone());
    let second = RecordingRemoteServer::spawn_success("builder-b", events);
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder(&first.node_id, &first.base_url, "secret", "direct"),
            RemoteInventoryRecord::builder(&second.node_id, &second.base_url, "secret", "direct"),
        ],
    );

    let check = root_label("check");
    let lint = root_label("lint");
    let docs = root_label("docs");
    let spec = cascade_workspace(&workspace, &check, &[lint.clone(), docs.clone()]);
    let summary = run_tasks(
        &spec,
        std::slice::from_ref(&check),
        &RunOptions {
            jobs: 2,
            ..RunOptions::default()
        },
    )
    .await
    .expect("remote cascade run");

    let lint_node = result_node(&summary, &lint);
    let docs_node = result_node(&summary, &docs);
    assert_ne!(lint_node, docs_node);
}
