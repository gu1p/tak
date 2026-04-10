use super::*;
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
            let image = container.image.trim();
            if image.is_empty() {
                bail!("invalid_submit_fields: execution.runtime.container.image is required");
            }
            Ok(RemoteRuntimeSpec::Containerized {
                image: image.to_string(),
            })
        }
        None => bail!("invalid_submit_fields: execution.runtime.kind is required"),
    }
}
