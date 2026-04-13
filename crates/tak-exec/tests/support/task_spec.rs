#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;

use tak_core::model::{
    CurrentStateSpec, OutputSelectorSpec, RemoteSpec, RemoteTransportKind, StepDef, TaskLabel,
};

use super::workspace_task_spec::build_remote_task_spec;

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
) -> (tak_core::model::WorkspaceSpec, TaskLabel) {
    remote_task_spec_with_context_and_outputs(
        workspace_root,
        name,
        steps,
        remote,
        CurrentStateSpec::default(),
        Vec::new(),
    )
}

pub fn remote_task_spec_with_context(
    workspace_root: &Path,
    name: &str,
    steps: Vec<StepDef>,
    remote: RemoteSpec,
    context: CurrentStateSpec,
) -> (tak_core::model::WorkspaceSpec, TaskLabel) {
    remote_task_spec_with_context_and_outputs(
        workspace_root,
        name,
        steps,
        remote,
        context,
        Vec::new(),
    )
}

pub fn remote_task_spec_with_outputs(
    workspace_root: &Path,
    name: &str,
    steps: Vec<StepDef>,
    remote: RemoteSpec,
    outputs: Vec<OutputSelectorSpec>,
) -> (tak_core::model::WorkspaceSpec, TaskLabel) {
    remote_task_spec_with_context_and_outputs(
        workspace_root,
        name,
        steps,
        remote,
        CurrentStateSpec::default(),
        outputs,
    )
}

pub fn remote_task_spec_with_context_and_outputs(
    workspace_root: &Path,
    name: &str,
    steps: Vec<StepDef>,
    remote: RemoteSpec,
    context: CurrentStateSpec,
    outputs: Vec<OutputSelectorSpec>,
) -> (tak_core::model::WorkspaceSpec, TaskLabel) {
    build_remote_task_spec(workspace_root, name, steps, remote, context, outputs)
}
