use super::*;
use tak_proto::WorkspaceUploadRef;

pub(crate) struct RemoteSubmitPayloadInput<'a> {
    pub(crate) target: &'a StrictRemoteTarget,
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) task: &'a ResolvedTask,
    pub(crate) remote_workspace: &'a RemoteWorkspaceStage,
    pub(crate) session: Option<&'a crate::engine::session_workspaces::PreparedTaskSession>,
    pub(crate) execution_label: Option<&'a str>,
    pub(crate) fused_members: Option<&'a [ResolvedTask]>,
    pub(crate) fused_member_execution_labels: Option<&'a BTreeMap<TaskLabel, String>>,
    pub(crate) workspace_upload: Option<&'a WorkspaceUploadRef>,
}

pub(crate) fn build_remote_submit_payload(
    input: RemoteSubmitPayloadInput<'_>,
) -> Result<SubmitTaskRequest> {
    let RemoteSubmitPayloadInput {
        target,
        task_run_id,
        attempt,
        task,
        remote_workspace,
        session,
        execution_label,
        fused_members,
        fused_member_execution_labels,
        workspace_upload,
    } = input;
    let _ = &remote_workspace.manifest_hash;
    let metadata = task_run_metadata_for_runtime(task, target.runtime.as_ref());
    Ok(SubmitTaskRequest {
        task_run_id: task_run_id.to_string(),
        attempt,
        workspace_zip: match workspace_upload {
            Some(_) => Vec::new(),
            None => base64::engine::general_purpose::STANDARD
                .decode(&remote_workspace.archive_zip_base64)
                .context("failed decoding staged workspace archive")?,
        },
        steps: task
            .steps
            .iter()
            .map(step_submit_value)
            .collect::<Result<Vec<_>>>()?,
        timeout_s: task.timeout_s,
        runtime: target.runtime.as_ref().map(remote_runtime_submit_value),
        task_label: task.label.to_string(),
        needs: task.needs.iter().map(need_submit_value).collect(),
        outputs: task
            .outputs
            .iter()
            .map(output_selector_submit_value)
            .collect(),
        session: session.map(session_submit_value),
        origin: Some(metadata.origin),
        runtime_source: metadata.runtime_source,
        command: metadata.command,
        fused_members: fused_members
            .unwrap_or(&[])
            .iter()
            .map(|member| {
                fused_members::fused_member_submit_value(member, fused_member_execution_labels)
            })
            .collect::<Result<Vec<_>>>()?,
        execution_label: execution_label.map(str::to_string),
        workspace_upload: workspace_upload.cloned(),
    })
}

fn session_submit_value(
    session: &crate::engine::session_workspaces::PreparedTaskSession,
) -> ExecutionSession {
    let share_paths = match &session.reuse {
        tak_core::model::SessionReuseSpec::ShareWorkspace => Vec::new(),
        tak_core::model::SessionReuseSpec::SharePaths { paths } => {
            paths.iter().map(output_selector_submit_value).collect()
        }
        tak_core::model::SessionReuseSpec::Container => Vec::new(),
    };
    ExecutionSession {
        key: session.key.clone(),
        name: session.name.clone(),
        reuse: session.reuse.as_str().to_string(),
        share_paths,
    }
}

pub(super) fn step_submit_value(step_def: &StepDef) -> Result<Step> {
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
        RemoteRuntimeSpec::Containerized {
            source,
            resource_limits,
        } => RuntimeSpec {
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
                resource_limits: resource_limits
                    .as_ref()
                    .and_then(proto_resource_limits_value),
            })),
        },
    }
}

fn proto_resource_limits_value(
    limits: &tak_core::model::ContainerResourceLimitsSpec,
) -> Option<ContainerResourceLimits> {
    Some(ContainerResourceLimits {
        cpu_cores: limits.cpu_cores?,
        memory_mb: limits.memory_mb?,
    })
}

fn need_submit_value(need: &NeedDef) -> SubmittedNeed {
    SubmittedNeed {
        name: need.limiter.name.clone(),
        scope: scope_value(&need.limiter.scope).to_string(),
        scope_key: need.limiter.scope_key.clone(),
        slots: need.slots,
    }
}

fn output_selector_submit_value(selector: &OutputSelectorSpec) -> tak_proto::OutputSelector {
    tak_proto::OutputSelector {
        kind: Some(match selector {
            OutputSelectorSpec::Path(path) => {
                tak_proto::output_selector::Kind::Path(path.path.clone())
            }
            OutputSelectorSpec::Glob { pattern } => {
                tak_proto::output_selector::Kind::Glob(pattern.clone())
            }
        }),
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
