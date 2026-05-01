use std::collections::{BTreeMap, HashMap};
use std::fs;

use tak_core::model::*;
use tak_exec::{RunOptions, run_tasks};

use crate::support::shell_step;

#[tokio::test]
async fn cascaded_execution_clears_dependency_legacy_session() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("workspace");
    let (spec, target) = workspace_with_legacy_session_dependency(&workspace_root);

    run_tasks(&spec, std::slice::from_ref(&target), &RunOptions::default())
        .await
        .expect("cascade should run");

    assert!(workspace_root.join("dep-root.txt").exists());
}

fn workspace_with_legacy_session_dependency(root: &std::path::Path) -> (WorkspaceSpec, TaskLabel) {
    let dep_label = label("dep");
    let target_label = label("check");
    let stale_session = session_use("old-session");
    let mut dep = base_task(dep_label.clone());
    dep.steps = vec![shell_step("pwd > dep-root.txt")];
    dep.execution = TaskExecutionSpec::UseSession {
        name: stale_session.name.clone(),
        cascade: false,
    };
    dep.session = Some(stale_session);
    let mut target = base_task(target_label.clone());
    target.deps = vec![dep_label.clone()];
    target.steps = vec![shell_step("true")];
    target.cascade_execution = true;
    let mut tasks = BTreeMap::new();
    tasks.insert(dep_label, dep);
    tasks.insert(target_label.clone(), target);
    (
        WorkspaceSpec {
            project_id: "tak-test".into(),
            root: root.to_path_buf(),
            tasks,
            sessions: BTreeMap::new(),
            limiters: HashMap::new(),
            queues: HashMap::new(),
        },
        target_label,
    )
}

fn base_task(label: TaskLabel) -> ResolvedTask {
    ResolvedTask {
        label,
        doc: String::new(),
        deps: Vec::new(),
        steps: Vec::new(),
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: None,
        execution: TaskExecutionSpec::LocalOnly(LocalSpec::default()),
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    }
}

fn session_use(name: &str) -> SessionUseSpec {
    SessionUseSpec {
        name: name.to_string(),
        display_name: name.to_string(),
        execution: Some(Box::new(TaskExecutionSpec::LocalOnly(LocalSpec::default()))),
        reuse: SessionReuseSpec::ShareWorkspace,
        context: None,
    }
}

fn label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}
