use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use tak_core::model::{
    CurrentStateSpec, RemoteRuntimeSpec, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec,
    TaskLabel, WorkspaceSpec,
};
use tak_make::{ContainerSource, ExecutionPlacement, GoalAnnotations, GoalExecutionRequest};

use crate::cli::run_override_runtime::explicit_container_runtime_override;
use crate::cli::run_overrides_support::{RunPlacementSelector, rewrite_execution_for_placement};

pub(super) fn make_workspace(request: GoalExecutionRequest) -> Result<(WorkspaceSpec, TaskLabel)> {
    let GoalExecutionRequest {
        workspace_root,
        makefile_path,
        argv,
        annotations,
    } = request;
    let label = make_label();
    let task = make_task(
        label.clone(),
        makefile_path.display().to_string(),
        argv,
        annotations,
    )?;
    let tasks = BTreeMap::from([(label.clone(), task)]);
    let workspace = WorkspaceSpec {
        project_id: "tak-make".to_string(),
        root: workspace_root,
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    };
    Ok((workspace, label))
}

fn make_task(
    label: TaskLabel,
    makefile_path: String,
    argv: Vec<String>,
    annotations: GoalAnnotations,
) -> Result<ResolvedTask> {
    Ok(ResolvedTask {
        label,
        doc: format!("Synthetic Make invocation from `{makefile_path}`."),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv,
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: None,
        execution: annotated_execution(&annotations)?,
        session: None,
        cascade_execution: false,
        tags: vec!["make".to_string()],
    })
}

fn annotated_execution(annotations: &GoalAnnotations) -> Result<TaskExecutionSpec> {
    let runtime = annotation_runtime(annotations)?;
    let placement = match annotations.placement {
        Some(ExecutionPlacement::Remote) => RunPlacementSelector::Remote,
        Some(ExecutionPlacement::Local) | None => RunPlacementSelector::Local,
    };
    if annotations.placement.is_none() && runtime.is_none() {
        return Ok(TaskExecutionSpec::default());
    }
    Ok(rewrite_execution_for_placement(
        &TaskExecutionSpec::default(),
        placement,
        runtime,
    ))
}

fn annotation_runtime(annotations: &GoalAnnotations) -> Result<Option<RemoteRuntimeSpec>> {
    match annotations.container.as_ref() {
        None => Ok(None),
        Some(ContainerSource::Image { image }) => {
            explicit_container_runtime_override(Some(image), None, None)
        }
        Some(ContainerSource::Dockerfile {
            dockerfile,
            build_context,
        }) => explicit_container_runtime_override(None, Some(dockerfile), build_context.as_deref()),
    }
}

fn make_label() -> TaskLabel {
    TaskLabel {
        package: "//".to_string(),
        name: "make".to_string(),
    }
}
