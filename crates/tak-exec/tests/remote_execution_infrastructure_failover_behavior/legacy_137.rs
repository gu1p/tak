use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    LockedEnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord,
    remote_builder_spec, remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn legacy_infrastructure_137_does_not_fail_over_or_submit_to_replacement() {
    let mut env = LockedEnvGuard::acquire();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    let first_events = RecordingEvents::default();
    let second_events = RecordingEvents::default();
    let first =
        RecordingRemoteServer::spawn_legacy_infrastructure_137("builder-a", first_events.clone());
    let second = RecordingRemoteServer::spawn_success("builder-b", second_events.clone());
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder("builder-a", &first.base_url, "secret", "direct"),
            RemoteInventoryRecord::builder("builder-b", &second.base_url, "secret", "direct"),
        ],
    );
    let (spec, label) = remote_task_spec(
        &workspace,
        "legacy-137",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );

    let error = run_tasks(&spec, &[label], &RunOptions::default())
        .await
        .expect_err("legacy infrastructure 137 is terminal");

    assert!(format!("{error:#}").contains("builder-a failed with exit 137"));
    assert_eq!(super::submit_count(&first_events), 1);
    assert_eq!(super::submit_count(&second_events), 0);
}
