#![allow(clippy::await_holding_lock)]

use crate::support::{
    EnvGuard, RemoteInventoryRecord, env_lock, shell_step, write_remote_inventory,
};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use tak_core::model::*;
use tak_exec::{PlacementMode, RunOptions, run_tasks};
#[tokio::test]
async fn execution_policy_falls_back_to_local_before_remote_task_start() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "unreachable",
            "not a valid endpoint",
            "secret",
            "direct",
        )],
    );
    let (spec, label) = policy_workspace(&workspace_root);
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("policy fallback should run locally");
    let result = summary.results.get(&label).expect("summary result");
    assert_eq!(result.placement_mode, PlacementMode::Local);
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/policy.txt")).expect("local output"),
        "local-fallback\n"
    );
}
fn policy_workspace(root: &std::path::Path) -> (WorkspaceSpec, TaskLabel) {
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
                ExecutionPlacementSpec::Remote(remote_builder()),
                ExecutionPlacementSpec::Local(LocalSpec::default()),
            ],
        },
        session: None,
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
fn remote_builder() -> RemoteSpec {
    RemoteSpec {
        pool: Some("build".into()),
        required_tags: vec!["builder".into()],
        required_capabilities: vec!["linux".into()],
        transport_kind: RemoteTransportKind::Direct,
        runtime: None,
        selection: RemoteSelectionSpec::Sequential,
    }
}
fn image_runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: "alpine:3.20".into(),
        },
    }
}
