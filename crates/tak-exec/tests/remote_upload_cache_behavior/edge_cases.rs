use tak_exec::{RunOptions, run_tasks};

use super::helpers::{recording_node, workspace_with_identical_tasks};
use crate::support::{EnvGuard, RecordingEvents, RecordingRemoteServer, env_lock};

#[tokio::test]
async fn legacy_node_without_uploads_falls_back_to_inline_submit() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());

    let events = RecordingEvents::default();
    let node = RecordingRemoteServer::spawn_success_legacy_inline("builder-legacy", events.clone());
    recording_node(&config, &node);

    let (spec, labels) = workspace_with_identical_tasks(&workspace, 1);
    run_tasks(
        &spec,
        &labels,
        &RunOptions {
            jobs: 1,
            ..RunOptions::default()
        },
    )
    .await
    .expect("remote task should run against a legacy node");

    assert_eq!(
        events.upload_begin_count(),
        0,
        "a legacy node must not see uploads"
    );
    let payloads = events.submit_payloads();
    assert_eq!(payloads.len(), 1);
    assert!(
        payloads[0].workspace_upload.is_none(),
        "legacy submit must not reference an upload"
    );
    assert!(
        !payloads[0].workspace_zip.is_empty(),
        "legacy submit must inline the workspace zip"
    );
}

#[tokio::test]
async fn reaped_blob_triggers_reupload_and_succeeds() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());

    let events = RecordingEvents::default();
    // This node reaps each upload right after a submit references it, so the second task's
    // reused-ref submit gets a 409 and the client must re-upload.
    let node = RecordingRemoteServer::spawn_success_reaping_uploads("builder-a", events.clone());
    recording_node(&config, &node);

    let (spec, labels) = workspace_with_identical_tasks(&workspace, 2);
    run_tasks(
        &spec,
        &labels,
        &RunOptions {
            jobs: 1,
            ..RunOptions::default()
        },
    )
    .await
    .expect("both tasks should succeed despite the reaped blob");

    let begins = events.upload_begin_ids();
    assert_eq!(
        begins.len(),
        2,
        "the second task must re-upload after its reused blob was reaped"
    );
    // The reused reference (the first task's upload) is the one rejected with 409, which drives
    // the client's invalidate-and-re-upload fallback.
    assert_eq!(
        events.upload_conflicts(),
        vec![begins[0].clone()],
        "exactly the reaped first-upload reference must be rejected with 409"
    );
}
