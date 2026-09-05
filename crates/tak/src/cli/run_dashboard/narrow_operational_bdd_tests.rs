use super::render_test_support::frame_at_size;
use super::test_support::state;

#[test]
fn narrow_tasks_keep_node_attempt_and_cache_values_reachable() {
    let rendered = frame_at_size(&state(), 48, 48);
    let tasks = rendered
        .split_once("TASKS")
        .and_then(|(_, rest)| rest.split_once("SCHEDULER QUEUE"))
        .map(|(tasks, _)| tasks)
        .expect("tasks panel");

    for expected in ["NODE worker-a", "TRY 1", "CACHE miss"] {
        assert!(
            tasks.contains(expected),
            "narrow task metadata {expected:?} is unreachable:\n{rendered}"
        );
    }
}

#[test]
fn narrow_footer_keeps_the_complete_cancellation_help_reachable() {
    let rendered = frame_at_size(&state(), 48, 48);

    assert!(
        rendered.contains("Ctrl-C cancel/again detach"),
        "narrow cancellation help is clipped:\n{rendered}"
    );
}
