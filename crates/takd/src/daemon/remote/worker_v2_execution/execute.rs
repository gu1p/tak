use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use tak_core::label::parse_label;
use tak_core::model::StepDef;
use tak_core::v2::{ResolvedTaskUnit, Step};
use tak_proto::worker_v2::{DispatchAttemptRequest, WorkerAttemptIdentity, WorkerOutputStream};
use tak_runner::{
    OutputStream, RemoteWorkerExecutionSpec, RunCancellation, TaskOutputChunk, TaskOutputObserver,
    execute_remote_worker_steps_with_output_and_cancellation,
};

use super::super::{RemoteNodeContext, SubmitAttemptStore};
use super::workspace::PreparedWorkspace;

pub(super) enum ExecutionOutcome {
    Succeeded {
        runtime_kind: Option<String>,
        runtime_engine: Option<String>,
    },
    Failed {
        exit_code: Option<i32>,
        runtime_kind: Option<String>,
        runtime_engine: Option<String>,
    },
}

pub(super) async fn run(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    request: &DispatchAttemptRequest,
    prepared: &PreparedWorkspace,
    cancellation: &RunCancellation,
) -> Result<ExecutionOutcome> {
    let mut runtime_kind = None;
    let mut runtime_engine = None;
    for task in &request.payload.tasks {
        let observer: Arc<dyn TaskOutputObserver> = Arc::new(DurableObserver {
            store: store.clone(),
            identity: request.identity.clone(),
            task_id: task.task_id.clone(),
        });
        let result = execute_remote_worker_steps_with_output_and_cancellation(
            &prepared.workspace_root,
            &spec(context, request, task, prepared)?,
            Some(observer),
            cancellation,
        )
        .await?;
        if !result.success {
            return Ok(ExecutionOutcome::Failed {
                exit_code: result.exit_code,
                runtime_kind: result.runtime_kind.clone(),
                runtime_engine: result.runtime_engine.clone(),
            });
        }
        runtime_kind = result.runtime_kind.clone();
        runtime_engine = result.runtime_engine.clone();
        super::outputs::publish(store, &request.identity, task, &prepared.workspace_root)?;
    }
    Ok(ExecutionOutcome::Succeeded {
        runtime_kind,
        runtime_engine,
    })
}

fn spec(
    context: &RemoteNodeContext,
    request: &DispatchAttemptRequest,
    task: &ResolvedTaskUnit,
    prepared: &PreparedWorkspace,
) -> Result<RemoteWorkerExecutionSpec> {
    Ok(RemoteWorkerExecutionSpec {
        task_label: parse_label(&task.task_id, "//")?,
        task_run_id: format!(
            "{}/{}/{}",
            request.identity.run_id, request.identity.job_id, task.task_id
        ),
        attempt: request.identity.authored_attempt,
        steps: task.steps.iter().map(step).collect(),
        base_environment: environment(request, task, prepared)?,
        clear_environment: true,
        timeout_s: task.timeout_s,
        runtime: super::super::super::task_runtime::runner_runtime(
            task.runtime.as_ref(),
            prepared
                .home
                .parent()
                .ok_or_else(|| anyhow::anyhow!("worker attempt HOME has no private root"))?,
        )?,
        node_id: request.identity.node_id.clone(),
        container_user: None,
        image_cache: context
            .image_cache_config()
            .map(|config| config.runner_options()),
        container_identity: None,
    })
}

fn environment(
    request: &DispatchAttemptRequest,
    task: &ResolvedTaskUnit,
    prepared: &PreparedWorkspace,
) -> Result<BTreeMap<String, String>> {
    let identity = &request.identity;
    let mut values = BTreeMap::from([
        ("HOME".into(), prepared.home.display().to_string()),
        ("TMPDIR".into(), prepared.temporary.display().to_string()),
        ("TMP".into(), prepared.temporary.display().to_string()),
        ("TEMP".into(), prepared.temporary.display().to_string()),
        ("TAK_RUN_ID".into(), identity.run_id.clone()),
        ("TAK_JOB_ID".into(), identity.job_id.clone()),
        ("TAK_TASK_ID".into(), task.task_id.clone()),
        ("TAK_NODE_ID".into(), identity.node_id.clone()),
        ("TAK_ATTEMPT".into(), identity.authored_attempt.to_string()),
        (
            "TAK_DISPATCH_GENERATION".into(),
            identity.dispatch_generation.to_string(),
        ),
        (
            "TAK_WORKSPACE".into(),
            prepared.workspace_root.display().to_string(),
        ),
    ]);
    super::super::super::task_runtime::insert_host_path_for_native_runtime(
        &mut values,
        task.runtime.as_ref(),
    );
    for name in &task.pass_env_names {
        let value = request
            .payload
            .environment_values
            .iter()
            .find(|value| value.name == *name)
            .ok_or_else(|| anyhow::anyhow!("passed environment `{name}` is missing"))?;
        values.insert(name.clone(), value.value.clone());
    }
    Ok(values)
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

struct DurableObserver {
    store: SubmitAttemptStore,
    identity: WorkerAttemptIdentity,
    task_id: String,
}

impl TaskOutputObserver for DurableObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        let stream = match chunk.stream {
            OutputStream::Stdout => WorkerOutputStream::Stdout,
            OutputStream::Stderr => WorkerOutputStream::Stderr,
        };
        self.store
            .append_worker_v2_event(&self.identity, &self.task_id, stream, &chunk.bytes)?;
        Ok(())
    }
}
