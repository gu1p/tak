use std::collections::{BTreeMap, HashMap};

use crate::support::{remote_builder_spec, shell_step};
use tak_core::model::{
    ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec::Image, CurrentStateSpec,
    ExecutionPlacementSpec, LimiterKey, QueueDef, RemoteRuntimeSpec::Containerized,
    RemoteSelectionSpec, RemoteTransportKind, ResolvedTask, RetryDef, TaskExecutionSpec, TaskLabel,
    WorkspaceSpec,
};

pub(super) fn cascade_workspace(
    root: &std::path::Path,
    check: &TaskLabel,
    deps: &[TaskLabel],
) -> WorkspaceSpec {
    let mut tasks = BTreeMap::new();
    for dep in deps {
        tasks.insert(dep.clone(), remote_task(dep, Vec::new(), false));
    }
    tasks.insert(check.clone(), remote_task(check, deps.to_vec(), true));
    WorkspaceSpec {
        project_id: "remote-cascade-distribution-test".into(),
        root: root.to_path_buf(),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    }
}

fn remote_task(label: &TaskLabel, deps: Vec<TaskLabel>, cascade_execution: bool) -> ResolvedTask {
    let mut remote = remote_builder_spec(RemoteTransportKind::Direct);
    remote.selection = RemoteSelectionSpec::Shuffle;
    ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps,
        steps: (!cascade_execution)
            .then(|| shell_step("true"))
            .into_iter()
            .collect(),
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
        execution: remote_execution(remote, cascade_execution),
        session: None,
        cascade_execution,
        tags: Vec::new(),
    }
}

fn remote_execution(
    remote: tak_core::model::RemoteSpec,
    cascade_execution: bool,
) -> TaskExecutionSpec {
    if cascade_execution {
        return TaskExecutionSpec::ByExecutionPolicy {
            name: "remote-check-policy".into(),
            placements: vec![ExecutionPlacementSpec::Remote(remote)],
        };
    }
    TaskExecutionSpec::RemoteOnly(remote)
}

pub(super) fn root_label(name: &str) -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: name.into(),
    }
}

pub(super) fn result_node(summary: &tak_exec::RunSummary, label: &TaskLabel) -> String {
    summary
        .results
        .get(label)
        .and_then(|result| result.remote_node_id.clone())
        .expect("remote result node")
}
