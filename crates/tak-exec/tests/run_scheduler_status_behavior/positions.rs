use std::sync::Arc;

use tak_exec::{RunOptions, TaskStatusEventKind, run_tasks};

use super::support::{Events, label, workspace};

#[tokio::test]
async fn scheduler_reports_visible_units_and_recomputes_ready_positions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(Events::default());
    run_tasks(
        &workspace(temp.path()),
        &[label("all")],
        &RunOptions {
            jobs: 1,
            output_observer: Some(observer.clone()),
            ..RunOptions::default()
        },
    )
    .await
    .expect("run tasks");

    let events = observer.0.lock().expect("events");
    let planned = events
        .iter()
        .filter(|event| event.kind == TaskStatusEventKind::TaskPlanned)
        .map(|event| event.task_label.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(planned, vec!["a", "b"]);
    let positions = events
        .iter()
        .filter(|event| event.queue_id.as_deref() == Some("scheduler"))
        .filter_map(|event| {
            event
                .queue_position
                .map(|position| (event.task_label.name.as_str(), position))
        })
        .collect::<Vec<_>>();
    assert!(positions.contains(&("a", 1)), "{positions:?}");
    assert!(positions.contains(&("b", 2)), "{positions:?}");
    assert!(positions.contains(&("b", 1)), "{positions:?}");
}
