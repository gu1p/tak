use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use tak_core::label::parse_label;
use tak_core::model::{
    BackoffDef, CurrentStateSpec, LocalSpec, OutputSelectorSpec, PathAnchor, PathRef, ResolvedTask,
    RetryDef, StepDef, TaskExecutionSpec, WorkspaceSpec,
};
use tak_core::v2::{AuthoredTask, OutputSelector, Step};

use super::V2AuthoredRoot;

pub(super) fn read_only(root: V2AuthoredRoot) -> Result<WorkspaceSpec> {
    let tasks = root
        .module
        .tasks
        .iter()
        .map(task)
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok(WorkspaceSpec {
        project_id: root.module.project_id.unwrap_or_else(|| "tak-v2".into()),
        root: root.workspace_root,
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    })
}

fn task(authored: &AuthoredTask) -> Result<(tak_core::model::TaskLabel, ResolvedTask)> {
    let label = parse_label(&authored.name, "//")?;
    let dependencies = authored
        .deps
        .iter()
        .map(|dependency| parse_label(dependency, &label.package))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let resolved = ResolvedTask {
        label: label.clone(),
        doc: authored.doc.clone(),
        deps: dependencies,
        steps: authored.steps.iter().map(step).collect(),
        needs: Vec::new(),
        queue: None,
        retry: retry(authored),
        timeout_s: authored.timeout_s,
        context: CurrentStateSpec::default(),
        outputs: authored.outputs.iter().map(output).collect(),
        container_runtime: None,
        execution: TaskExecutionSpec::LocalOnly(LocalSpec::default()),
        session: None,
        cascade_execution: false,
        tags: authored.tags.clone(),
    };
    Ok((label, resolved))
}

fn step(value: &Step) -> StepDef {
    match value {
        Step::Cmd { argv, cwd, env } => StepDef::Cmd {
            argv: argv.clone(),
            cwd: cwd.clone(),
            env: env.clone(),
        },
        Step::Script {
            path,
            argv,
            interpreter,
            cwd,
            env,
        } => StepDef::Script {
            path: path.clone(),
            argv: argv.clone(),
            interpreter: interpreter.clone(),
            cwd: cwd.clone(),
            env: env.clone(),
        },
    }
}

fn output(value: &OutputSelector) -> OutputSelectorSpec {
    match value {
        OutputSelector::Path { value } => OutputSelectorSpec::Path(PathRef {
            anchor: PathAnchor::Workspace,
            path: value.clone(),
        }),
        OutputSelector::Glob { value } => OutputSelectorSpec::Glob {
            pattern: value.clone(),
        },
    }
}

fn retry(task: &AuthoredTask) -> RetryDef {
    let retry = task.retry.as_ref();
    RetryDef {
        attempts: retry.map_or(1, |value| value.max_attempts.get()),
        on_exit: Vec::new(),
        backoff: BackoffDef::Fixed {
            seconds: retry.map_or(0.0, |value| value.backoff_millis as f64 / 1_000.0),
        },
    }
}
