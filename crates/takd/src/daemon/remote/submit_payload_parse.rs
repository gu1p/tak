use super::*;
use tak_core::model::{ContainerRuntimeSourceSpec, PathAnchor, PathRef, normalize_path_ref};
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
        runtime: request
            .runtime
            .as_ref()
            .map(parse_remote_worker_runtime_spec)
            .transpose()?,
    })
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
