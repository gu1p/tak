use std::collections::BTreeMap;
use std::sync::Arc;

use tak_core::model::StepDef;
use tak_exec::{RunOptions, TaskStatusEventKind, run_tasks};

use super::support::{Events, label, workspace};

#[tokio::test]
async fn fail_fast_marks_ready_but_undispatched_work_cancelled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let observer = Arc::new(Events::default());
    let mut spec = workspace(temp.path());
    spec.tasks.get_mut(&label("a")).expect("task a").steps = vec![StepDef::Cmd {
        argv: vec!["sh".into(), "-c".into(), "exit 7".into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    let result = run_tasks(
        &spec,
        &[label("all")],
        &RunOptions {
            jobs: 1,
            output_observer: Some(observer.clone()),
            ..RunOptions::default()
        },
    )
    .await;
    assert!(result.is_err(), "fail-fast run should fail");

    let events = observer.0.lock().expect("events");
    let cancelled = events
        .iter()
        .filter(|event| event.kind == TaskStatusEventKind::Cancellation)
        .map(|event| event.task_label.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(cancelled, vec!["b"]);
}
