use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use tak_core::model::*;

use crate::support::shell_step;

pub(super) fn policy_workspace(
    root: &Path,
    transport_kind: RemoteTransportKind,
) -> (WorkspaceSpec, TaskLabel) {
    let label = TaskLabel {
        package: "//".into(),
        name: "check".into(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![shell_step(
            "mkdir -p out && echo local-fallback > out/policy.txt",
        )],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: Some(image_runtime()),
        execution: TaskExecutionSpec::ByExecutionPolicy {
            name: "remote-or-local".into(),
            placements: vec![
                ExecutionPlacementSpec::Remote(remote_builder(transport_kind)),
                ExecutionPlacementSpec::Local(LocalSpec::default()),
            ],
        },
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    };
    let mut tasks = BTreeMap::new();
    tasks.insert(label.clone(), task);
    (
        WorkspaceSpec {
            project_id: "tak-test".into(),
            root: root.to_path_buf(),
            tasks,
            sessions: BTreeMap::new(),
            limiters: HashMap::new(),
            queues: HashMap::new(),
        },
        label,
    )
}

fn remote_builder(transport_kind: RemoteTransportKind) -> RemoteSpec {
    RemoteSpec {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()],
        transport_kind,
        runtime: None,
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    }
}

fn image_runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: "alpine:3.20".into(),
        },
        resource_limits: None,
    }
}
