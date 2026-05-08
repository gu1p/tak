use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec, ResolvedTask, StepDef};

use crate::engine::PlacementMode;
use crate::engine::remote_models::TaskPlacement;

#[derive(Debug, Clone)]
pub(crate) struct TaskRunMetadata {
    pub(crate) origin: String,
    pub(crate) runtime: Option<String>,
    pub(crate) runtime_source: Option<String>,
    pub(crate) command: Option<String>,
}

pub(crate) fn task_run_metadata_for_placement(
    task: &ResolvedTask,
    placement: &TaskPlacement,
) -> TaskRunMetadata {
    task_run_metadata_for_runtime(task, placement_runtime(placement))
}

pub(crate) fn task_run_metadata_for_runtime(
    task: &ResolvedTask,
    runtime: Option<&RemoteRuntimeSpec>,
) -> TaskRunMetadata {
    TaskRunMetadata {
        origin: task_origin(task),
        runtime: runtime.map(runtime_kind),
        runtime_source: runtime.map(runtime_source_display),
        command: task.steps.first().and_then(step_command_display),
    }
}

fn task_origin(task: &ResolvedTask) -> String {
    if task.tags.iter().any(|tag| tag == "docker-run") {
        return "docker-run".to_string();
    }
    if task.tags.iter().any(|tag| tag == "exec") {
        return "exec".to_string();
    }
    "task".to_string()
}

fn placement_runtime(placement: &TaskPlacement) -> Option<&RemoteRuntimeSpec> {
    match placement.placement_mode {
        PlacementMode::Local => placement
            .local
            .as_ref()
            .and_then(|local| local.runtime.as_ref()),
        PlacementMode::Remote => placement
            .strict_remote_target
            .as_ref()
            .and_then(|target| target.runtime.as_ref())
            .or_else(|| {
                placement
                    .remote
                    .as_ref()
                    .and_then(|remote| remote.runtime.as_ref())
            }),
    }
}

fn runtime_kind(runtime: &RemoteRuntimeSpec) -> String {
    match runtime {
        RemoteRuntimeSpec::Containerized { .. } => "containerized".to_string(),
    }
}

fn runtime_source_display(runtime: &RemoteRuntimeSpec) -> String {
    match runtime {
        RemoteRuntimeSpec::Containerized { source } => match source {
            ContainerRuntimeSourceSpec::Image { image } => format!("image:{image}"),
            ContainerRuntimeSourceSpec::Dockerfile { dockerfile, .. } => {
                format!("dockerfile:{}", dockerfile.path)
            }
        },
    }
}

fn step_command_display(step: &StepDef) -> Option<String> {
    match step {
        StepDef::Cmd { argv, .. } => argv_display(argv),
        StepDef::Script {
            path,
            argv,
            interpreter,
            ..
        } => {
            let mut full = Vec::new();
            if let Some(interpreter) = interpreter {
                full.push(interpreter.clone());
            }
            full.push(path.clone());
            full.extend(argv.clone());
            argv_display(&full)
        }
    }
}

fn argv_display(argv: &[String]) -> Option<String> {
    if argv.is_empty() {
        return None;
    }
    Some(
        argv.iter()
            .map(|arg| shell_token(arg))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn shell_token(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || b"/._-:=@%+".contains(&byte))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}
