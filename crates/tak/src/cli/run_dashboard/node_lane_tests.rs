use super::model::DashboardState;
use super::test_support::seed;

#[test]
fn node_lanes_flatten_fused_tasks_and_repeat_ready_work_for_each_candidate() {
    let mut seed = seed();
    seed.jobs[0].task_ids = vec!["//app:compile".into(), "//app:bundle".into()];
    let state = DashboardState::new(seed);

    assert_eq!(
        state.nodes["worker-a"].active_jobs,
        ["//app:compile", "//app:bundle"]
    );
    assert_eq!(
        candidates(&state, "worker-a"),
        [("//:lint", Some("builds"))]
    );
    assert_eq!(
        candidates(&state, "worker-b"),
        [("//:lint", Some("builds"))]
    );
}

#[test]
fn assigned_blocked_or_retry_waiting_work_is_not_a_candidate_queue_entry() {
    let mut seed = seed();
    seed.jobs[2].node_id = Some("worker-a".into());
    seed.jobs.push(super::model::DashboardJobSeed {
        job_id: "blocked".into(),
        task_ids: vec!["//:blocked".into()],
        state: "blocked".into(),
        node_id: None,
        candidate_node_ids: vec!["worker-a".into()],
        queue: Some("builds".into()),
        attempt: 0,
        cache: None,
    });
    seed.jobs.push(super::model::DashboardJobSeed {
        job_id: "retrying".into(),
        task_ids: vec!["//:retrying".into()],
        state: "retrying".into(),
        node_id: None,
        candidate_node_ids: vec!["worker-a".into()],
        queue: Some("builds".into()),
        attempt: 1,
        cache: None,
    });
    let state = DashboardState::new(seed);

    assert!(candidates(&state, "worker-a").is_empty());
    assert!(candidates(&state, "worker-b").is_empty());
}

fn candidates<'a>(state: &'a DashboardState, node: &str) -> Vec<(&'a str, Option<&'a str>)> {
    state.nodes[node]
        .candidate_queue
        .iter()
        .map(|entry| (entry.task.as_str(), entry.queue.as_deref()))
        .collect()
}
