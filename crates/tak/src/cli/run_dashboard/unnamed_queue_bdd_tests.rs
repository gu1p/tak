use super::model::DashboardState;
use super::render_test_support::frame;
use super::test_support::{node_frame, seed};

#[test]
fn unnamed_queue_is_consistent_without_claiming_a_capacity_failure() {
    let mut seed = seed();
    seed.jobs[2].queue = None;
    let state = DashboardState::new(seed);
    let rendered = frame(&state, 160);
    assert!(rendered.contains("queue: none"), "{rendered}");
    assert!(!rendered.contains("queue: unavailable"), "{rendered}");
    let nodes = node_frame(&state, 160, 48);
    assert!(nodes.contains("queue=none"), "{nodes}");
    assert!(!nodes.contains("queue=default"), "{nodes}");
    for label in ["NODES", "TASKS", "SCHEDULER QUEUE", "LIVE LOGS"] {
        assert!(rendered.contains(label), "missing {label}: {rendered}");
    }
}
