use crate::support;

use std::path::Path;

use tak_proto::{ActiveJob, NodeStatusResponse, SubmittedNeed};
use takd::agent::{InitAgentOptions, init_agent, read_config};

#[test]
fn tasks_renders_live_job_details_and_fallbacks_from_control_socket_status() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (config_root, state_root) = support::cli::roots(temp.path());
    init_direct_agent(&config_root, &state_root);
    let mut status = support::takd_tasks::empty_status("builder-a");
    status.active_jobs.push(ActiveJob {
        task_run_id: "task-run-1".into(),
        attempt: 2,
        task_label: "//apps/web:fmt".into(),
        execution_label: Some("check.fmt".into()),
        started_at_ms: i64::MAX,
        needs: vec![SubmittedNeed {
            name: "gpu".into(),
            scope: "project".into(),
            scope_key: Some("workspace-a".into()),
            slots: 2.5,
        }],
        execution_root_bytes: 1536,
        runtime: Some("containerized".into()),
        ..Default::default()
    });
    status.active_jobs.push(ActiveJob {
        task_run_id: "task-run-2".into(),
        attempt: 1,
        task_label: "   ".into(),
        execution_label: Some(" ".into()),
        started_at_ms: i64::MAX,
        needs: Vec::new(),
        execution_root_bytes: 42,
        runtime: None,
        ..Default::default()
    });
    let stdout = run_tasks_with_status(&config_root, &state_root, status);
    for needle in [
        "node=builder-a",
        "task_label=check.fmt",
        "task_run_id=task-run-1",
        "attempt=2",
        "age=0s",
        "needs=gpu(project/workspace-a)=2.5",
        "exec_root=1.5KiB",
        "runtime=containerized",
        "task_label=(unknown)",
        "needs=(none)",
        "exec_root=42B",
        "runtime=none",
    ] {
        assert!(stdout.contains(needle), "{stdout}");
    }
    assert!(
        !stdout.contains("task_label=//apps/web:fmt"),
        "raw task label should not be primary when execution label exists:\n{stdout}"
    );
}

fn run_tasks_with_status(
    config_root: &Path,
    state_root: &Path,
    status: NodeStatusResponse,
) -> String {
    let socket = support::takd_tasks::spawn_status_socket(
        state_root,
        &read_config(config_root).expect("read config").bearer_token,
        status,
    );
    let output = support::takd_tasks::run_takd_tasks(config_root, state_root);
    assert!(output.status.success(), "takd tasks should succeed");
    socket.join().expect("fake control socket exits");
    String::from_utf8(output.stdout).expect("takd tasks stdout should be utf8")
}

fn init_direct_agent(config_root: &Path, state_root: &Path) {
    init_agent(
        config_root,
        state_root,
        InitAgentOptions {
            node_id: Some("builder-a"),
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
