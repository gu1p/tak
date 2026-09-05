use super::model::{DashboardJobSeed, DashboardSeed, DashboardState};
use super::test_support::node_frame;

#[test]
fn each_node_lane_lists_all_active_tasks_and_its_candidate_ready_queue() {
    let rendered = node_frame(&state(), 96, 42);
    let nodes = rendered.split("TASKS").next().expect("nodes precede tasks");

    for expected in [
        "//applications/frontend:compile-production-assets",
        "//app:bundle",
    ] {
        assert!(
            nodes.contains(expected),
            "missing active row {expected}:\n{nodes}"
        );
    }
    assert_eq!(nodes.matches("CANDIDATE QUEUE").count(), 2, "{nodes}");
    assert_eq!(
        nodes
            .split_whitespace()
            .collect::<String>()
            .matches("//checks/quality:lint-entire-workspace")
            .count(),
        2,
        "{nodes}"
    );
    assert!(
        nodes.contains("worker-a") && nodes.contains("worker-b"),
        "{nodes}"
    );
}

#[test]
fn narrow_node_lanes_wrap_instead_of_hiding_task_names() {
    let rendered = node_frame(&state(), 34, 48);
    let nodes = rendered.split("TASKS").next().expect("nodes precede tasks");
    let compact = nodes
        .chars()
        .filter(|character| !character.is_whitespace() && *character != '│')
        .collect::<String>();

    for task in [
        "//applications/frontend:compile-production-assets",
        "//app:bundle",
        "//checks/quality:lint-entire-workspace",
    ] {
        assert!(compact.contains(task), "clipped {task}:\n{nodes}");
    }
    assert!(nodes.contains("CANDIDATE QUEUE"), "{nodes}");
}

fn state() -> DashboardState {
    DashboardState::new(DashboardSeed {
        run_id: "run-node-lanes".into(),
        lifecycle: "running".into(),
        max_parallel_jobs: 2,
        jobs: vec![
            job(
                "fused",
                &[
                    "//applications/frontend:compile-production-assets",
                    "//app:bundle",
                ],
                "running",
                Some("worker-a"),
                &["worker-a", "worker-b"],
            ),
            job(
                "lint",
                &["//checks/quality:lint-entire-workspace"],
                "ready",
                None,
                &["worker-a", "worker-b"],
            ),
        ],
    })
}

fn job(
    id: &str,
    tasks: &[&str],
    state: &str,
    node: Option<&str>,
    candidates: &[&str],
) -> DashboardJobSeed {
    DashboardJobSeed {
        job_id: id.into(),
        task_ids: tasks.iter().map(|task| (*task).into()).collect(),
        state: state.into(),
        node_id: node.map(str::to_owned),
        candidate_node_ids: candidates.iter().map(|node| (*node).into()).collect(),
        queue: Some("builds".into()),
        attempt: u32::from(node.is_some()),
        cache: None,
    }
}
