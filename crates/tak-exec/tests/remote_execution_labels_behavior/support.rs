use std::collections::{BTreeMap, HashMap};

use tak_core::model::{
    ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec::Image, CurrentStateSpec, LimiterKey,
    QueueDef, RemoteRuntimeSpec::Containerized, RemoteSelectionSpec, RemoteTransportKind,
    ResolvedTask, RetryDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};

use crate::support::{remote_builder_spec, shell_step};

pub(super) fn workspace_with_dependency(
    root: &std::path::Path,
    check: &TaskLabel,
    fmt: &TaskLabel,
) -> WorkspaceSpec {
    let mut tasks = BTreeMap::new();
    tasks.insert(fmt.clone(), remote_task(fmt, Vec::new()));
    tasks.insert(check.clone(), remote_task(check, vec![fmt.clone()]));
    WorkspaceSpec {
        project_id: "remote-execution-labels-test".into(),
        root: root.to_path_buf(),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    }
}

pub(super) fn workspace_with_shared_dependency(
    root: &std::path::Path,
    first: &TaskLabel,
    second: &TaskLabel,
    shared: &TaskLabel,
) -> WorkspaceSpec {
    let mut tasks = BTreeMap::new();
    tasks.insert(shared.clone(), remote_task(shared, Vec::new()));
    tasks.insert(first.clone(), remote_task(first, vec![shared.clone()]));
    tasks.insert(second.clone(), remote_task(second, vec![shared.clone()]));
    WorkspaceSpec {
        project_id: "remote-execution-labels-test".into(),
        root: root.to_path_buf(),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    }
}

fn remote_task(label: &TaskLabel, deps: Vec<TaskLabel>) -> ResolvedTask {
    let mut remote = remote_builder_spec(RemoteTransportKind::Direct);
    remote.selection = RemoteSelectionSpec::Shuffle;
    ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps,
        steps: vec![shell_step("true")],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: Some(Containerized {
            source: Image {
                image: "alpine:3.20".into(),
            },
            resource_limits: Some(ContainerResourceLimitsSpec {
                cpu_cores: Some(1.0),
                memory_mb: Some(512),
            }),
        }),
        execution: TaskExecutionSpec::RemoteOnly(remote),
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    }
}

pub(super) fn root_label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}
