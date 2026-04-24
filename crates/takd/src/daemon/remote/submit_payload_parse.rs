use super::*;
use tak_core::model::{
    ContainerRuntimeSourceSpec, OutputSelectorSpec, PathAnchor, PathRef, normalize_path_ref,
};
use tak_proto::{RuntimeSpec, Step, runtime_spec, step};

pub(super) fn parse_remote_worker_submit_payload(
    request: &tak_proto::SubmitTaskRequest,
) -> Result<RemoteWorkerSubmitPayload> {
    Ok(RemoteWorkerSubmitPayload {
        workspace_zip: request.workspace_zip.clone(),
        task_label: request.task_label.clone(),
        attempt: request.attempt,
        steps: request
            .steps
            .iter()
            .map(parse_remote_worker_step)
            .collect::<Result<Vec<_>>>()?,
        timeout_s: request.timeout_s,
        runtime: Some(
            request
                .runtime
                .as_ref()
                .ok_or_else(|| anyhow!("invalid_submit_fields: execution.runtime is required")) // fail closed against host-level remote execution
                .and_then(parse_remote_worker_runtime_spec)?,
        ),
        outputs: request
            .outputs
            .iter()
            .map(parse_remote_worker_output_selector)
            .collect::<Result<Vec<_>>>()?,
        session: request
            .session
            .as_ref()
            .map(parse_remote_worker_session)
            .transpose()?,
    })
}

fn parse_remote_worker_session(
    session: &tak_proto::ExecutionSession,
) -> Result<RemoteWorkerSession> {
    let key = session.key.trim().to_string();
    if key.is_empty() {
        bail!("invalid_submit_fields: session.key is required");
    }
    let reuse = match session.reuse.as_str() {
        "share_workspace" => RemoteWorkerSessionReuse::ShareWorkspace,
        "share_paths" => {
            if session.share_paths.is_empty() {
                bail!("invalid_submit_fields: session.share_paths cannot be empty");
            }
            RemoteWorkerSessionReuse::SharePaths {
                paths: session
                    .share_paths
                    .iter()
                    .map(parse_remote_worker_output_selector)
                    .collect::<Result<Vec<_>>>()?,
            }
        }
        other => bail!("invalid_submit_fields: unsupported session.reuse `{other}`"),
    };
    Ok(RemoteWorkerSession { key, reuse })
}

fn parse_remote_worker_step(step: &Step) -> Result<StepDef> {
    match step.kind.as_ref() {
        Some(step::Kind::Cmd(cmd)) => Ok(StepDef::Cmd {
            argv: cmd.argv.clone(),
            cwd: cmd.cwd.clone(),
            env: cmd.env.clone().into_iter().collect(),
        }),
        Some(step::Kind::Script(script)) => Ok(StepDef::Script {
            path: script.path.clone(),
            argv: script.argv.clone(),
            interpreter: script.interpreter.clone(),
            cwd: script.cwd.clone(),
            env: script.env.clone().into_iter().collect(),
        }),
        None => bail!("invalid_submit_fields: step.kind is required"),
    }
}

fn parse_remote_worker_runtime_spec(value: &RuntimeSpec) -> Result<RemoteRuntimeSpec> {
    match value.kind.as_ref() {
        Some(runtime_spec::Kind::Container(container)) => {
            match (
                container
                    .image
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                container
                    .dockerfile
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            ) {
                (Some(_), Some(_)) => bail!(
                    "invalid_submit_fields: execution.runtime.container must specify exactly one source"
                ),
                (None, None) => bail!(
                    "invalid_submit_fields: execution.runtime.container must specify exactly one source"
                ),
                (Some(image), None) => Ok(RemoteRuntimeSpec::Containerized {
                    source: ContainerRuntimeSourceSpec::Image {
                        image: image.to_string(),
                    },
                }),
                (None, Some(dockerfile)) => {
                    let dockerfile = normalize_workspace_submit_path(
                        dockerfile,
                        "execution.runtime.container.dockerfile",
                    )?;
                    let build_context = normalize_workspace_submit_path(
                        container.build_context.as_deref().unwrap_or("."),
                        "execution.runtime.container.build_context",
                    )?;
                    Ok(RemoteRuntimeSpec::Containerized {
                        source: ContainerRuntimeSourceSpec::Dockerfile {
                            dockerfile,
                            build_context,
                        },
                    })
                }
            }
        }
        None => bail!("invalid_submit_fields: execution.runtime.kind is required"),
    }
}

fn normalize_workspace_submit_path(value: &str, field: &str) -> Result<PathRef> {
    let normalized = normalize_path_ref("workspace", value)
        .map_err(|err| anyhow!("invalid_submit_fields: {field} {err}"))?;
    if normalized.anchor != PathAnchor::Workspace {
        bail!("invalid_submit_fields: {field} must be workspace-anchored");
    }
    Ok(normalized)
}

fn parse_remote_worker_output_selector(
    selector: &tak_proto::OutputSelector,
) -> Result<OutputSelectorSpec> {
    match selector.kind.as_ref() {
        Some(tak_proto::output_selector::Kind::Path(path)) => Ok(OutputSelectorSpec::Path(
            normalize_workspace_submit_path(path, "outputs.path")?,
        )),
        Some(tak_proto::output_selector::Kind::Glob(pattern)) => Ok(OutputSelectorSpec::Glob {
            pattern: normalize_workspace_submit_glob(pattern)?,
        }),
        None => bail!("invalid_submit_fields: outputs.kind is required"),
    }
}

fn normalize_workspace_submit_glob(value: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("invalid_submit_fields: outputs.glob cannot be empty");
    }
    if normalized.starts_with('@') {
        bail!("invalid_submit_fields: outputs repo anchors are not supported in V1");
    }
    if normalized.starts_with("//") || normalized.starts_with('/') {
        bail!("invalid_submit_fields: outputs glob must be workspace-relative");
    }
    if normalized.split('/').any(|segment| segment == "..") {
        bail!("invalid_submit_fields: outputs glob cannot escape workspace");
    }
    let normalized = normalized.replace('\\', "/");
    let mut builder = GitignoreBuilder::new(".");
    builder
        .add_line(None, &normalized)
        .map_err(|err| anyhow!("invalid_submit_fields: outputs.glob {err}"))?;
    Ok(normalized)
}
