use std::collections::BTreeMap;
use std::sync::Arc;

use tak_core::model::StepDef;
use tak_exec::{OutputStream, execute_remote_worker_steps_with_output_and_cancellation};

use crate::support::{CollectingObserver, worker_spec};

#[tokio::test]
async fn worker_can_clear_ambient_environment_and_preserve_task_run_identity() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let ambient_name = std::env::vars()
        .map(|(name, _)| name)
        .find(|name| !matches!(name.as_str(), "PATH" | "HOME" | "TMPDIR" | "TMP" | "TEMP"))
        .expect("test process has an ambient variable");
    let mut spec = worker_spec(
        "clear_environment",
        vec![StepDef::Cmd {
            argv: vec!["/usr/bin/env".into()],
            cwd: None,
            env: BTreeMap::from([("STEP_VALUE".into(), "step".into())]),
        }],
        None,
        None,
        "local",
    );
    spec.task_run_id = "run/job/attempt".into();
    spec.clear_environment = true;
    spec.base_environment = BTreeMap::from([
        ("PATH".into(), "/usr/bin:/bin".into()),
        ("BASE_VALUE".into(), "base".into()),
    ]);
    let observer = Arc::new(CollectingObserver::default());
    let result = execute_remote_worker_steps_with_output_and_cancellation(
        temp.path(),
        &spec,
        Some(observer.clone()),
        &tak_exec::RunCancellation::default(),
    )
    .await
    .unwrap();
    assert!(result.success);
    let chunks = observer.snapshot();
    assert!(
        chunks
            .iter()
            .all(|chunk| chunk.task_run_id == spec.task_run_id)
    );
    let output = chunks
        .iter()
        .filter(|chunk| chunk.stream == OutputStream::Stdout)
        .flat_map(|chunk| &chunk.bytes)
        .copied()
        .collect::<Vec<_>>();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("BASE_VALUE=base") && output.contains("STEP_VALUE=step"));
    assert!(
        !output
            .lines()
            .any(|line| line.starts_with(&format!("{ambient_name}=")))
    );
}

#[test]
fn worker_spec_debug_redacts_passed_environment_values() {
    let mut spec = worker_spec("redacted", vec![], None, None, "local");
    spec.base_environment = BTreeMap::from([("TOKEN".into(), "never-debug-this".into())]);

    let debug = format!("{spec:?}");
    assert!(debug.contains("TOKEN"), "{debug}");
    assert!(!debug.contains("never-debug-this"), "{debug}");
}
