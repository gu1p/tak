use std::collections::BTreeSet;
use std::path::Path;

use tak_core::model::{
    CurrentStateOrigin, CurrentStateSpec, PathAnchor, PathRef, RemoteTransportKind, ResolvedTask,
    TaskLabel, WorkspaceSpec,
};

use crate::support::{
    RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

pub(super) fn seed_repo(workspace_root: &Path) {
    std::fs::create_dir_all(workspace_root.join("src")).expect("src dir");
    std::fs::write(workspace_root.join("src/main.rs"), b"fn main() {}\n").expect("main.rs");
    std::fs::write(
        workspace_root.join("Cargo.toml"),
        b"[package]\nname = \"demo\"\n",
    )
    .expect("Cargo.toml");
}

/// A workspace of `count` independent remote tasks that all stage the same content — modelling
/// the per-task re-uploads a cascading job would otherwise perform against one node.
pub(super) fn workspace_with_identical_tasks(
    workspace_root: &Path,
    count: usize,
) -> (WorkspaceSpec, Vec<TaskLabel>) {
    seed_repo(workspace_root);
    let (mut spec, first) = remote_task_spec(
        workspace_root,
        "task-0",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let template: ResolvedTask = spec.tasks.get(&first).expect("template task").clone();
    let mut labels = vec![first.clone()];
    for index in 1..count {
        let label = TaskLabel {
            package: first.package.clone(),
            name: format!("task-{index}"),
        };
        let mut task = template.clone();
        task.label = label.clone();
        spec.tasks.insert(label.clone(), task);
        labels.push(label);
    }
    (spec, labels)
}

pub(super) fn context_under(root_dir: &str) -> CurrentStateSpec {
    CurrentStateSpec {
        roots: vec![PathRef {
            anchor: PathAnchor::Workspace,
            path: root_dir.to_string(),
        }],
        ignored: Vec::new(),
        include: Vec::new(),
        origin: CurrentStateOrigin::ImplicitDefault,
    }
}

pub(super) fn recording_node(config: &Path, server: &RecordingRemoteServer) {
    write_remote_inventory(
        config,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            "secret",
            "direct",
        )],
    );
}

/// Asserts that every recorded submit referenced an uploaded blob (no inline fallback) and that
/// they all referenced the SAME `upload_id` — i.e. the tasks reused one upload.
pub(super) fn assert_all_reuse_single_upload(events: &RecordingEvents, expected_submits: usize) {
    let payloads = events.submit_payloads();
    assert_eq!(payloads.len(), expected_submits, "unexpected submit count");
    let mut upload_ids = BTreeSet::new();
    for payload in &payloads {
        let upload = payload
            .workspace_upload
            .as_ref()
            .expect("each task must reference an uploaded workspace, not inline it");
        assert!(
            payload.workspace_zip.is_empty(),
            "a reused submit must not also inline the workspace zip"
        );
        upload_ids.insert(upload.upload_id.clone());
    }
    assert_eq!(
        upload_ids.len(),
        1,
        "all tasks should reference one cached upload: {upload_ids:?}"
    );
}
