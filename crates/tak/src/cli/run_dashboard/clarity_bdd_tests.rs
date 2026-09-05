use super::model::LogLine;
use super::render_test_support::frame_at_size;
use super::test_support::state;

#[test]
fn ordinary_terminal_leaves_room_for_output_without_repeating_progress() {
    let mut state = state();
    state.logs = (0..12)
        .map(|index| LogLine {
            job: "//:build".into(),
            node: "worker-a".into(),
            text: format!("output-{index:02}"),
        })
        .collect();
    let rendered = frame_at_size(&state, 100, 24);
    assert!(
        rendered.contains("output-06") && rendered.contains("output-11"),
        "{rendered}"
    );
    assert_eq!(rendered.matches("complete").count(), 1, "{rendered}");
    assert!(rendered.contains("1 queued"), "{rendered}");
    assert!(!rendered.contains("candidate rows"), "{rendered}");
    assert!(!rendered.contains("1 active tasks"), "{rendered}");
}

#[test]
fn long_task_names_and_metadata_remain_readable_at_normal_width() {
    let mut state = state();
    state.jobs.get_mut("build").unwrap().task_ids =
        vec!["//applications/frontend:compile-production-assets".into()];
    state.jobs.get_mut("build").unwrap().node_id =
        Some("production-worker-with-a-long-name".into());
    let rendered = frame_at_size(&state, 80, 40);
    let tasks = rendered
        .split_once("TASKS")
        .unwrap()
        .1
        .split_once("SCHEDULER QUEUE")
        .unwrap()
        .0;
    let compact = tasks.split_whitespace().collect::<String>();
    for expected in [
        "//applications/frontend:compile-production-assets",
        "production-worker-with-a-long-name",
        "miss",
    ] {
        assert!(
            compact.contains(expected),
            "clipped {expected}:\n{rendered}"
        );
    }
}

#[test]
fn narrow_output_wraps_to_reveal_the_end_of_an_error() {
    let mut state = state();
    state.logs.push(LogLine {
        job: "//applications/frontend:build".into(),
        node: "production-worker".into(),
        text:
            "Compilation failed because the imported module could not be found: missing_module.rs"
                .into(),
    });
    let rendered = frame_at_size(&state, 48, 40);
    let logs = rendered.split_once("LIVE LOGS").unwrap().1;
    assert!(logs.contains("missing_module.rs"), "{rendered}");
}
