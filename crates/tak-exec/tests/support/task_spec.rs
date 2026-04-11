#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use tak_core::model::{
    CurrentStateSpec, LimiterKey, QueueDef, RemoteSpec, RemoteTransportKind, ResolvedTask,
    RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};

pub fn shell_step(script: &str) -> StepDef {
    StepDef::Cmd {
        argv: vec!["sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }
}

pub fn remote_builder_spec(transport_kind: RemoteTransportKind) -> RemoteSpec {
    RemoteSpec {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()],
        transport_kind,
        runtime: None,
    }
}

pub fn remote_task_spec(
    workspace_root: &Path,
    name: &str,
    steps: Vec<StepDef>,
    remote: RemoteSpec,
) -> (WorkspaceSpec, TaskLabel) {
    let label = TaskLabel {
        package: "//apps/web".into(),
        name: name.into(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps,
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        container_runtime: None,
        execution: TaskExecutionSpec::RemoteOnly(remote),
        tags: Vec::new(),
    };
    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);
    (
        WorkspaceSpec {
            project_id: "tak-test".into(),
            root: workspace_root.to_path_buf(),
            tasks,
            limiters: HashMap::<LimiterKey, _>::new(),
            queues: HashMap::<LimiterKey, QueueDef>::new(),
        },
        label,
    )
}
