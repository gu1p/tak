use std::collections::{BTreeMap, HashMap};

use tak_core::model::{
    CurrentStateSpec, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};
use tak_exec::{RunOptions, run_tasks};

#[tokio::test]
async fn keep_going_skips_failed_descendants_but_runs_independent_tasks() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let tasks = BTreeMap::from([
        task("failed", vec![], "exit 23"),
        task("blocked", vec![label("failed")], "touch blocked"),
        task("survivor", vec![], "touch survivor"),
    ]);
    let spec = WorkspaceSpec {
        project_id: "keep-going".into(),
        root: workspace.clone(),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    };

    let error = run_tasks(
        &spec,
        &[label("blocked"), label("survivor")],
        &RunOptions {
            jobs: 3,
            keep_going: true,
            ..RunOptions::default()
        },
    )
    .await
    .expect_err("failed branch should fail the run");

    assert!(error.to_string().contains("one or more tasks failed"));
    assert!(workspace.join("survivor").exists());
    assert!(!workspace.join("blocked").exists());
}

fn task(name: &str, deps: Vec<TaskLabel>, script: &str) -> (TaskLabel, ResolvedTask) {
    let label = label(name);
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps,
        steps: vec![StepDef::Cmd {
            argv: vec!["sh".into(), "-c".into(), script.into()],
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
    };
    (label, task)
}

fn label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}
