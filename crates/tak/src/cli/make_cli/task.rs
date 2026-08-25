use std::collections::{BTreeMap, HashMap};

use anyhow::{Result, anyhow};
use tak_core::model::{
    CurrentStateSpec, RemoteRuntimeSpec, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec,
    TaskLabel, WorkspaceSpec,
};
use tak_make::{
    ContainerSource, ExecutionPlacement, GoalAnnotations, GoalExecutionRequest, MakeExecutionPlan,
    ParallelOutputMode,
};

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
        Vec::new(),
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

pub(super) struct ParallelMakeWorkspace {
    pub(super) spec: WorkspaceSpec,
    pub(super) root: TaskLabel,
    pub(super) goals: Vec<ParallelMakeGoal>,
}

pub(super) struct ParallelMakeGoal {
    pub(super) label: TaskLabel,
    pub(super) goal: String,
    pub(super) output: ParallelOutputMode,
}

pub(super) fn parallel_make_workspace(plan: MakeExecutionPlan) -> Result<ParallelMakeWorkspace> {
    let labels = plan
        .goals
        .iter()
        .enumerate()
        .map(|(index, goal)| (goal.goal.clone(), parallel_make_label(index)))
        .collect::<BTreeMap<_, _>>();
    let mut tasks = BTreeMap::new();
    let mut goals = Vec::new();
    for goal in plan.goals {
        let label = goal_label(&labels, &goal.goal)?;
        let dependencies = goal
            .dependencies
            .iter()
            .map(|dependency| goal_label(&labels, dependency))
            .collect::<Result<Vec<_>>>()?;
        let task = make_task(
            label.clone(),
            plan.makefile_path.display().to_string(),
            goal.argv,
            goal.annotations,
            dependencies,
        )?;
        tasks.insert(label.clone(), task);
        goals.push(ParallelMakeGoal {
            label,
            goal: goal.goal,
            output: goal.parallel_output,
        });
    }
    let root = goal_label(&labels, &plan.root_goal)?;
    Ok(ParallelMakeWorkspace {
        spec: WorkspaceSpec {
            project_id: "tak-make".to_string(),
            root: plan.workspace_root,
            tasks,
            sessions: BTreeMap::new(),
            limiters: HashMap::new(),
            queues: HashMap::new(),
        },
        root,
        goals,
    })
}

fn goal_label(labels: &BTreeMap<String, TaskLabel>, goal: &str) -> Result<TaskLabel> {
    labels
        .get(goal)
        .cloned()
        .ok_or_else(|| anyhow!("missing synthetic Make goal `{goal}`"))
}

fn make_task(
    label: TaskLabel,
    makefile_path: String,
    argv: Vec<String>,
    annotations: GoalAnnotations,
    dependencies: Vec<TaskLabel>,
) -> Result<ResolvedTask> {
    Ok(ResolvedTask {
        label,
        doc: format!("Synthetic Make invocation from `{makefile_path}`."),
        deps: dependencies,
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

fn parallel_make_label(index: usize) -> TaskLabel {
    TaskLabel {
        package: "//".to_string(),
        name: format!("make-{index}"),
    }
}
