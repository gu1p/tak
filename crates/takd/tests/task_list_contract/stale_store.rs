use crate::support;

use tak_proto::ActiveJob;
use takd::agent::{InitAgentOptions, init_agent, read_config};
use takd::daemon::remote::SubmitAttemptStore;

#[test]
fn tasks_uses_live_control_socket_not_unfinished_sqlite_rows() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = support::cli::roots(temp.path());
    init_direct_agent(&config_root, &state_root);
    let store = SubmitAttemptStore::with_db_path(state_root.join("agent.sqlite")).expect("store");
    store
        .register_submit_with_task_label(
            "stale-run",
            Some(1),
            "//apps/web:stale",
            "node-a",
            temp.path(),
        )
        .expect("register stale sqlite row");
    let socket = support::takd_tasks::spawn_status_socket(
        &state_root,
        &read_config(&config_root).expect("read config").bearer_token,
        support::takd_tasks::empty_status("node-a"),
    );

    let output = support::takd_tasks::run_takd_tasks(&config_root, &state_root);
    assert!(output.status.success(), "takd tasks should succeed");
    socket.join().expect("fake control socket exits");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Active Tasks"), "missing header:\n{stdout}");
    assert!(
        stdout.contains("(none)"),
        "missing empty live state:\n{stdout}"
    );
    assert!(
        !stdout.contains("stale-run"),
        "sqlite row leaked:\n{stdout}"
    );
}

#[test]
fn tasks_prints_execution_label_for_live_jobs_when_available() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = support::cli::roots(temp.path());
    init_direct_agent(&config_root, &state_root);
    let mut status = support::takd_tasks::empty_status("node-a");
    status.active_jobs.push(ActiveJob {
        task_run_id: "task-run-live".into(),
        attempt: 1,
        task_label: "//apps/web:fmt-check".into(),
        execution_label: Some("check.fmt-check".into()),
        started_at_ms: 1,
        needs: Vec::new(),
        execution_root_bytes: 0,
        runtime: Some("containerized".into()),
        origin: Some("task".into()),
        runtime_source: Some("image:alpine:3.20".into()),
        command: Some("true".into()),
        resource_limits: None,
    });
    let socket = support::takd_tasks::spawn_status_socket(
        &state_root,
        &read_config(&config_root).expect("read config").bearer_token,
        status,
    );

    let output = support::takd_tasks::run_takd_tasks(&config_root, &state_root);
    assert!(output.status.success(), "takd tasks should succeed");
    socket.join().expect("fake control socket exits");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("task_label=check.fmt-check"),
        "missing execution label:\n{stdout}"
    );
    assert!(
        !stdout.contains("task_label=//apps/web:fmt-check"),
        "raw task label should not be primary when execution label exists:\n{stdout}"
    );
}

fn init_direct_agent(config_root: &std::path::Path, state_root: &std::path::Path) {
    init_agent(
        config_root,
        state_root,
        InitAgentOptions {
            node_id: Some("node-a"),
            display_name: None,
            transport: Some("direct"),
            base_url: Some("http://127.0.0.1:43123"),
            pools: &[],
            tags: &[],
            capabilities: &[],
            image_cache_budget_percent: None,
            image_cache_budget_gb: None,
        },
    )
    .expect("init agent");
}
