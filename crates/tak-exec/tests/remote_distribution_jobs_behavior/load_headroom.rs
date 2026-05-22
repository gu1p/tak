use super::load_aware_support::{configure_workspace, run_remote_check};
use super::support::node_status;
use crate::support::{
    EnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord, env_lock,
    write_remote_inventory,
};

#[tokio::test]
async fn shuffled_remote_jobs_avoid_nodes_without_live_headroom() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let (_temp, workspace, config) = configure_workspace(&mut env);
    let events = RecordingEvents::default();
    let mut overloaded_status = node_status("builder-overloaded", 0, 0);
    overloaded_status
        .cpu
        .as_mut()
        .expect("cpu")
        .tak_admission_available_cores = Some(0.25);
    overloaded_status
        .memory
        .as_mut()
        .expect("memory")
        .tak_admission_available_bytes = Some(256 * 1024 * 1024);
    let overloaded = RecordingRemoteServer::spawn_success_with_status(
        "builder-overloaded",
        events.clone(),
        overloaded_status,
    );
    let mut fitting_status = node_status("builder-fitting", 1, 0);
    fitting_status
        .cpu
        .as_mut()
        .expect("cpu")
        .tak_admission_available_cores = Some(2.0);
    fitting_status
        .memory
        .as_mut()
        .expect("memory")
        .tak_admission_available_bytes = Some(2 * 1024 * 1024 * 1024);
    let fitting =
        RecordingRemoteServer::spawn_success_with_status("builder-fitting", events, fitting_status);
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder(
                &overloaded.node_id,
                &overloaded.base_url,
                "secret",
                "direct",
            ),
            RemoteInventoryRecord::builder(&fitting.node_id, &fitting.base_url, "secret", "direct"),
        ],
    );

    let selected = run_remote_check(&workspace).await;

    assert_eq!(selected.as_deref(), Some("builder-fitting"));
}
