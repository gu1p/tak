use super::model::{DashboardJobSeed, DashboardSeed, DashboardState};
use super::render_test_support::frame;

#[test]
fn persisted_precommit_job_is_visible_as_staging() {
    let state = DashboardState::new(DashboardSeed {
        run_id: "run-staged".into(),
        lifecycle: "awaiting_workspace".into(),
        max_parallel_jobs: 1,
        jobs: vec![DashboardJobSeed {
            job_id: "build".into(),
            task_ids: vec!["//:build".into()],
            state: "staged".into(),
            node_id: None,
            candidate_node_ids: vec!["worker-a".into()],
            queue: None,
            attempt: 0,
            cache: None,
        }],
    });

    let rendered = frame(&state, 80);
    let task_row = rendered
        .lines()
        .find(|line| line.contains("//:build"))
        .expect("staged task row remains visible");

    assert!(task_row.contains("staging"), "{task_row}");
    assert!(!task_row.contains("unknown"), "{task_row}");
}
