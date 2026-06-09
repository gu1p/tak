use tak_exec::{RunOptions, run_tasks};

use super::helpers::{
    assert_all_reuse_single_upload, recording_node, workspace_with_identical_tasks,
};
use crate::support::{EnvGuard, RecordingEvents, RecordingRemoteServer, env_lock};

#[tokio::test]
async fn cascading_job_uploads_identical_workspace_once() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());

    let events = RecordingEvents::default();
    let node = RecordingRemoteServer::spawn_success("builder-a", events.clone());
    recording_node(&config, &node);

    let (spec, labels) = workspace_with_identical_tasks(&workspace, 4);
    run_tasks(
        &spec,
        &labels,
        &RunOptions {
            jobs: 1,
            ..RunOptions::default()
        },
    )
    .await
    .expect("all four remote tasks should run");

    assert_eq!(
        events.upload_begin_count(),
        1,
        "four tasks sharing one workspace must upload to the node exactly once"
    );
    assert_all_reuse_single_upload(&events, 4);
}

#[tokio::test]
async fn concurrent_identical_tasks_single_flight_to_one_upload() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());

    let events = RecordingEvents::default();
    let node = RecordingRemoteServer::spawn_success("builder-a", events.clone());
    recording_node(&config, &node);

    let (spec, labels) = workspace_with_identical_tasks(&workspace, 4);
    // jobs > 1 runs the tasks concurrently; the cache's single-flight must still collapse the
    // identical-content uploads to one.
    run_tasks(
        &spec,
        &labels,
        &RunOptions {
            jobs: 4,
            ..RunOptions::default()
        },
    )
    .await
    .expect("all four concurrent remote tasks should run");

    assert_eq!(
        events.upload_begin_count(),
        1,
        "concurrent identical-content tasks must single-flight to one upload"
    );
    assert_all_reuse_single_upload(&events, 4);
}
