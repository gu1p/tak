use super::model::LogLine;
use super::render_test_support::frame_at_size;
use super::test_support::state;

#[test]
fn node_lanes_name_the_tasks_running_there() {
    let rendered = super::test_support::node_frame(&state(), 118, 30);
    let node = rendered
        .split_once("worker-a")
        .and_then(|(_, rest)| rest.split_once("worker-b"))
        .map(|(node, _)| node)
        .expect("worker node lane");

    assert!(
        node.contains("ACTIVE") && node.contains("//:build"),
        "{node}"
    );
}

#[test]
fn live_log_panel_keeps_the_newest_multiline_output_visible() {
    let mut state = state();
    state.logs = vec![LogLine {
        job: "build".into(),
        node: "worker-a".into(),
        text: (1..=12)
            .map(|index| format!("line-{index:02}"))
            .collect::<Vec<_>>()
            .join("\n"),
    }];

    let rendered = frame_at_size(&state, 100, 24);

    assert!(
        rendered.contains("line-12"),
        "newest output is hidden:\n{rendered}"
    );
    assert!(
        !rendered.contains("line-01"),
        "old output displaced the tail:\n{rendered}"
    );
}
