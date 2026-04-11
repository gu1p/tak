use anyhow::{Context, Result, bail};
use base64::Engine;
use prost::Message;
use tak_core::model::{ContainerRuntimeSourceSpec, NeedDef, RemoteRuntimeSpec, ResolvedTask, Scope, StepDef};
use tak_proto::{
    CmdStep, ContainerRuntime, GetTaskResultResponse, PollTaskEventsResponse, RuntimeSpec,
    ScriptStep, Step, SubmitTaskRequest, SubmittedNeed, runtime_spec, step,
};

use crate::{
    OutputStream, ParsedRemoteEvents, RemoteLogChunk, RemoteWorkspaceStage, StrictRemoteTarget,
    SyncedOutput,
};

pub(crate) fn build_remote_submit_payload(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    task: &ResolvedTask,
    remote_workspace: &RemoteWorkspaceStage,
) -> Result<SubmitTaskRequest> {
    let _ = &remote_workspace.manifest_hash;
    Ok(SubmitTaskRequest {
        task_run_id: task_run_id.to_string(),
        attempt,
        workspace_zip: base64::engine::general_purpose::STANDARD
            .decode(&remote_workspace.archive_zip_base64)
            .context("failed decoding staged workspace archive")?,
        steps: task
            .steps
            .iter()
            .map(step_submit_value)
            .collect::<Result<Vec<_>>>()?,
        timeout_s: task.timeout_s,
        runtime: target.runtime.as_ref().map(remote_runtime_submit_value),
        task_label: task.label.to_string(),
        needs: task.needs.iter().map(need_submit_value).collect(),
    })
}

fn step_submit_value(step_def: &StepDef) -> Result<Step> {
    Ok(Step {
        kind: Some(match step_def {
            StepDef::Cmd { argv, cwd, env } => step::Kind::Cmd(CmdStep {
                argv: argv.clone(),
                cwd: cwd.clone(),
                env: env.clone().into_iter().collect(),
            }),
            StepDef::Script {
                path,
                argv,
                interpreter,
                cwd,
                env,
            } => step::Kind::Script(ScriptStep {
                path: path.clone(),
                argv: argv.clone(),
                interpreter: interpreter.clone(),
                cwd: cwd.clone(),
                env: env.clone().into_iter().collect(),
            }),
        }),
    })
}

fn remote_runtime_submit_value(runtime: &RemoteRuntimeSpec) -> RuntimeSpec {
    match runtime {
        RemoteRuntimeSpec::Containerized { source } => RuntimeSpec {
            kind: Some(runtime_spec::Kind::Container(ContainerRuntime {
                image: match source {
                    ContainerRuntimeSourceSpec::Image { image } => Some(image.clone()),
                    ContainerRuntimeSourceSpec::Dockerfile { .. } => None,
                },
                dockerfile: match source {
                    ContainerRuntimeSourceSpec::Image { .. } => None,
                    ContainerRuntimeSourceSpec::Dockerfile { dockerfile, .. } => {
                        Some(dockerfile.path.clone())
                    }
                },
                build_context: match source {
                    ContainerRuntimeSourceSpec::Image { .. } => None,
                    ContainerRuntimeSourceSpec::Dockerfile { build_context, .. } => {
                        Some(build_context.path.clone())
                    }
                },
            })),
        },
    }
}

fn need_submit_value(need: &NeedDef) -> SubmittedNeed {
    SubmittedNeed {
        name: need.limiter.name.clone(),
        scope: scope_value(&need.limiter.scope).to_string(),
        scope_key: need.limiter.scope_key.clone(),
        slots: need.slots,
    }
}

fn scope_value(scope: &Scope) -> &'static str {
    match scope {
        Scope::Machine => "machine",
        Scope::User => "user",
        Scope::Project => "project",
        Scope::Worktree => "worktree",
    }
}

#[cfg(test)]
mod submit_payload_behavior_tests;
#[cfg(test)]
mod submit_payload_error_tests;
#[cfg(test)]
mod submit_payload_test_support;
