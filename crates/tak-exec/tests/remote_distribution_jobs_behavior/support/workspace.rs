use std::collections::{BTreeMap, HashMap};

use crate::support::{remote_builder_spec, shell_step};
use tak_core::model::{
    ContainerResourceLimitsSpec, ContainerRuntimeSourceSpec::Image, CurrentStateSpec, LimiterKey,
    QueueDef, RemoteRuntimeSpec::Containerized, RemoteSelectionSpec, RemoteTransportKind,
    ResolvedTask, RetryDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};

pub(crate) fn remote_workspace(root: &std::path::Path, labels: &[TaskLabel]) -> WorkspaceSpec {
    let tasks = labels
        .iter()
        .map(|label| (label.clone(), remote_task(label)))
        .collect();
    WorkspaceSpec {
        project_id: "remote-jobs-test".into(),
        root: root.to_path_buf(),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    }
}

pub(crate) fn node_count(summary: &tak_exec::RunSummary, node_id: &str) -> usize {
    summary
        .results
        .values()
        .filter(|result| result.remote_node_id.as_deref() == Some(node_id))
        .count()
}

fn remote_task(label: &TaskLabel) -> ResolvedTask {
    let mut remote = remote_builder_spec(RemoteTransportKind::Direct);
    remote.selection = RemoteSelectionSpec::Shuffle;
    ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
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
