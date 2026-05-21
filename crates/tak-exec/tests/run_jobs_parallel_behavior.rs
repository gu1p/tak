use std::collections::{BTreeMap, HashMap};

use tak_core::model::{
    CurrentStateSpec, LimiterKey, QueueDef, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec,
    TaskLabel, WorkspaceSpec,
};
use tak_exec::{RunOptions, run_tasks};

#[tokio::test]
async fn independent_tasks_run_concurrently_up_to_jobs_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let spec = workspace_with_barrier_tasks(&workspace);
    let targets = ["a", "b"].map(label);

    let summary = run_tasks(
        &spec,
        &targets,
        &RunOptions {
            jobs: 2,
            ..RunOptions::default()
        },
    )
    .await
    .expect("independent tasks should pass when scheduled together");

    assert!(summary.results.values().all(|result| result.success));
}

fn workspace_with_barrier_tasks(root: &std::path::Path) -> WorkspaceSpec {
    let tasks = ["a", "b"]
        .into_iter()
        .map(|name| (label(name), barrier_task(name)))
        .collect();
    WorkspaceSpec {
        project_id: "jobs-test".into(),
        root: root.to_path_buf(),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    }
}

fn barrier_task(name: &str) -> ResolvedTask {
    let other = if name == "a" { "b" } else { "a" };
    let script = format!(
        "touch {name}.started; for _ in $(seq 1 100); do test -f {other}.started && exit 0; sleep 0.02; done; exit 7"
    );
    ResolvedTask {
        label: label(name),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".into(), "-c".into(), script],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: None,
        execution: TaskExecutionSpec::default(),
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    }
}

fn label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}
