use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use tak_core::model::{
    CurrentStateSpec, LimiterKey, OutputSelectorSpec, QueueDef, RemoteSpec, ResolvedTask, RetryDef,
    StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};

pub fn build_remote_task_spec(
    workspace_root: &Path,
    name: &str,
    steps: Vec<StepDef>,
    remote: RemoteSpec,
    context: CurrentStateSpec,
    outputs: Vec<OutputSelectorSpec>,
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
        context,
        outputs,
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
