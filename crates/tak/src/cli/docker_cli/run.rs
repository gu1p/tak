use std::collections::BTreeMap;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tak_core::model::{
    ContainerRuntimeSourceSpec, CurrentStateSpec, LocalSpec, RemoteRuntimeSpec,
    RemoteSelectionSpec, RemoteSpec, ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel,
    normalize_path_ref,
};
use tak_exec::{RunOptions, TaskOutputObserver, run_resolved_task};

use super::super::remote_inventory::RemoteRecord;
use super::super::run_output::StdStreamOutputObserver;
use super::super::task_history::{HistoryOutputObserver, TaskHistoryStore};
use super::DockerCliSelectors;
use super::run_spec::{DockerRunSpec, parse_docker_run};
use super::run_validate::validate_docker_run_spec;
use super::selectors::{matching_remotes, normalize_arch, normalize_os, selected_transport_kind};

pub(super) async fn run_docker_run(
    selectors: DockerCliSelectors,
    args: &[String],
) -> Result<ExitCode> {
    let spec = parse_docker_run(args)?;
    validate_docker_run_spec(&spec)?;

    let remotes = if selectors.local {
        Vec::new()
    } else {
        matching_remotes(&selectors)?
    };
    if remotes.is_empty() && !selectors.local {
        bail!("no configured remote agents match tak docker run");
    }

    let workspace_root = std::env::current_dir().context("failed to resolve current directory")?;
    let task = docker_run_task(&selectors, &spec, remotes.first())?;
    let result = run_resolved_task(
        &task,
        &workspace_root,
        &RunOptions {
            output_observer: Some(output_observer(selectors.local)?),
            ..RunOptions::default()
        },
    )
    .await?;

    if result.success {
        return Ok(ExitCode::SUCCESS);
    }

    Ok(exit_code_from_task_result(result.exit_code))
}

fn output_observer(local: bool) -> Result<Arc<dyn TaskOutputObserver>> {
    if local {
        return Ok(Arc::new(HistoryOutputObserver::new(
            TaskHistoryStore::open_default()?,
        )));
    }
    Ok(Arc::new(StdStreamOutputObserver::default()))
}

fn docker_run_task(
    selectors: &DockerCliSelectors,
    spec: &DockerRunSpec,
    selected_remote: Option<&RemoteRecord>,
) -> Result<ResolvedTask> {
    let runtime = docker_run_runtime(spec)?;
    Ok(ResolvedTask {
        label: TaskLabel {
            package: "//".to_string(),
            name: "docker-run".to_string(),
        },
        doc: "Synthetic container command created by `tak docker run`.".to_string(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: spec.argv.clone(),
            cwd: spec.workdir.clone(),
            env: parse_env(&spec.env)?,
        }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: Some(runtime.clone()),
        execution: docker_run_execution(selectors, runtime, selected_remote)?,
        session: None,
        cascade_execution: false,
        tags: vec!["docker".to_string(), "docker-run".to_string()],
    })
}

fn docker_run_runtime(spec: &DockerRunSpec) -> Result<RemoteRuntimeSpec> {
    let source = if let Some(image) = spec.image.as_ref() {
        ContainerRuntimeSourceSpec::Image {
            image: image.clone(),
        }
    } else {
        let dockerfile = spec
            .dockerfile
            .as_ref()
            .expect("validated dockerfile runtime");
        let build_context = spec.build_context.as_deref().unwrap_or(".");
        ContainerRuntimeSourceSpec::Dockerfile {
            dockerfile: normalize_path_ref("workspace", dockerfile)
                .with_context(|| format!("invalid Dockerfile path `{dockerfile}`"))?,
            build_context: normalize_path_ref("workspace", build_context)
                .with_context(|| format!("invalid build context path `{build_context}`"))?,
        }
    };
    Ok(RemoteRuntimeSpec::Containerized { source })
}

fn docker_run_execution(
    selectors: &DockerCliSelectors,
    runtime: RemoteRuntimeSpec,
    selected_remote: Option<&RemoteRecord>,
) -> Result<TaskExecutionSpec> {
    if selectors.local {
        return Ok(TaskExecutionSpec::LocalOnly(LocalSpec {
            runtime: Some(runtime),
            ..LocalSpec::default()
        }));
    }

    Ok(TaskExecutionSpec::RemoteOnly(RemoteSpec {
        pool: selectors.pool.clone(),
        required_tags: selectors.tags.clone(),
        required_capabilities: required_remote_capabilities(selectors, selected_remote),
        transport_kind: selected_transport_kind(selectors.transport.as_deref())?,
        runtime: Some(runtime),
        selection: RemoteSelectionSpec::Sequential,
        session: None,
    }))
}

fn required_remote_capabilities(
    selectors: &DockerCliSelectors,
    selected_remote: Option<&RemoteRecord>,
) -> Vec<String> {
    let mut capabilities = selectors.capabilities.clone();
    if let Some(arch) = selectors.arch.as_deref() {
        capabilities.push(format!("arch:{}", normalize_arch(arch)));
    }
    if let Some(os) = selectors.os.as_deref() {
        capabilities.push(format!("os:{}", normalize_os(os)));
    }
    if selectors.node.is_some()
        && let Some(remote) = selected_remote
    {
        capabilities.push(format!("node:{}", remote.node_id));
    }
    capabilities
}

fn parse_env(entries: &[String]) -> Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for entry in entries {
        let Some((key, value)) = entry.split_once('=') else {
            bail!("invalid --env value `{entry}`; expected KEY=VALUE");
        };
        if key.is_empty() {
            bail!("invalid --env value `{entry}`; key cannot be empty");
        }
        env.insert(key.to_string(), value.to_string());
    }
    Ok(env)
}

fn exit_code_from_task_result(code: Option<i32>) -> ExitCode {
    let normalized = code.unwrap_or(1).clamp(1, u8::MAX as i32) as u8;
    ExitCode::from(normalized)
}
