use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use tak_core::model::{
    ContainerRuntimeSourceSpec, CurrentStateSpec, RemoteRuntimeSpec, ResolvedTask, RetryDef,
    TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};

pub(super) fn image_runtime(image: &str) -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: image.to_string(),
        },
    }
}

pub(super) fn task_label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".to_string(),
        name: name.to_string(),
    }
}

pub(super) fn workspace_with_task(task: ResolvedTask) -> WorkspaceSpec {
    let mut tasks = BTreeMap::new();
    tasks.insert(task.label.clone(), task);
    WorkspaceSpec {
        project_id: "tak-test".to_string(),
        root: PathBuf::from("/tmp"),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

pub(super) fn resolved_task(label: TaskLabel, execution: TaskExecutionSpec) -> ResolvedTask {
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
        execution,
        session: None,
        tags: Vec::new(),
    }
}
