//! Behavioral tests for local executor ordering and retry contracts.

use std::collections::{BTreeMap, HashMap};
use std::fs;

use taskcraft_core::model::{
    BackoffDef, LimiterKey, NeedDef, QueueDef, QueueUseDef, ResolvedTask, RetryDef, StepDef,
    TaskLabel, WorkspaceSpec,
};
use taskcraft_exec::{RunOptions, run_tasks};

/// Constructs a minimal resolved task used by executor tests.
fn task(
    label: TaskLabel,
    deps: Vec<TaskLabel>,
    steps: Vec<StepDef>,
    retry: RetryDef,
) -> ResolvedTask {
    ResolvedTask {
        label,
        doc: String::new(),
        deps,
        steps,
        needs: Vec::<NeedDef>::new(),
        queue: Option::<QueueUseDef>::None,
        retry,
        timeout_s: None,
        tags: Vec::new(),
    }
}

/// Verifies dependency tasks execute before dependent targets.
#[tokio::test]
async fn executes_dependencies_before_target() {
    let temp = tempfile::tempdir().expect("tempdir");
    let log_file = temp.path().join("run.log");

    let build_label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "build".to_string(),
    };
    let test_label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "test".to_string(),
    };

    let build = task(
        build_label.clone(),
        Vec::new(),
        vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo build >> {}", log_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        RetryDef::default(),
    );

    let test = task(
        test_label.clone(),
        vec![build_label.clone()],
        vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo test >> {}", log_file.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        RetryDef::default(),
    );

    let mut tasks = BTreeMap::new();
    tasks.insert(build_label.clone(), build);
    tasks.insert(test_label.clone(), test);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, taskcraft_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    run_tasks(&spec, &[test_label], &RunOptions::default())
        .await
        .expect("run should succeed");

    let log = fs::read_to_string(log_file).expect("read log");
    assert_eq!(log.lines().collect::<Vec<_>>(), vec!["build", "test"]);
}

/// Verifies retry behavior when a task exits with a configured retriable exit code.
#[tokio::test]
async fn retries_failed_task_when_exit_code_matches_policy() {
    let temp = tempfile::tempdir().expect("tempdir");
    let marker = temp.path().join("first_attempt_seen");

    let label = TaskLabel {
        package: "//".to_string(),
        name: "flaky".to_string(),
    };

    let retry = RetryDef {
        attempts: 2,
        on_exit: vec![42],
        backoff: BackoffDef::Fixed { seconds: 0.0 },
    };

    let flaky = task(
        label.clone(),
        Vec::new(),
        vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!(
                    "if [ -f {0} ]; then exit 0; else touch {0}; exit 42; fi",
                    marker.display()
                ),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        retry,
    );

    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), flaky);

    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, taskcraft_core::model::LimiterDef>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, &[label], &RunOptions::default())
        .await
        .expect("run should succeed after retry");

    let result = summary.results.values().next().expect("result exists");
    assert_eq!(result.attempts, 2);
    assert!(result.success);
}
