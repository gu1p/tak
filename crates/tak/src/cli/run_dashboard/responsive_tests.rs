use super::model::{DashboardJobSeed, DashboardSeed, DashboardState};
use super::render_test_support::frame_at_size;

#[test]
fn large_runs_keep_every_operational_panel_reachable_on_a_short_terminal() {
    let jobs = (0..24)
        .map(|index| DashboardJobSeed {
            job_id: format!("job-{index}"),
            task_ids: vec![format!("//:task-{index}")],
            state: if index < 20 { "succeeded" } else { "running" }.into(),
            node_id: (index >= 20).then(|| format!("worker-{index}")),
            candidate_node_ids: vec![format!("worker-{index}")],
            queue: Some("builds".into()),
            attempt: 1,
            cache: None,
        })
        .collect();
    let state = DashboardState::new(DashboardSeed {
        run_id: "run-large".into(),
        lifecycle: "running".into(),
        max_parallel_jobs: 4,
        jobs,
    });

    let rendered = frame_at_size(&state, 100, 24);

    for expected in [
        "NODES",
        "worker-20",
        "TASKS",
        "STATE",
        "//:task-20",
        "SCHEDULER QUEUE",
        "Empty",
        "LIVE LOGS",
        "Waiting for task output",
        "Ctrl-C cancel",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected:?}:\n{rendered}"
        );
    }
}
