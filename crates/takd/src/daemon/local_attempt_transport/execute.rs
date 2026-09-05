use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use tak_core::label::parse_label;
use tak_core::model::StepDef;
use tak_core::v2::{ResolvedTaskUnit, Step};
use tak_runner::{
    OutputStream, RemoteWorkerExecutionOutcome, RemoteWorkerExecutionSpec, RunCancellation,
    TaskOutputChunk, TaskOutputObserver, execute_remote_worker_steps_with_output_and_cancellation,
};

use super::super::run_store::RunStore;
use super::super::run_store::execution::LocalExecutionSnapshot;
use super::super::scheduler::{
    AttemptCompletion, AttemptOutputStream, AttemptRuntimeMetadata, DispatchCommand,
};

pub(super) async fn run(
    store: &RunStore,
    command: &DispatchCommand,
    snapshot: &LocalExecutionSnapshot,
    workspace_root: &Path,
    cancellation: &RunCancellation,
) -> Result<AttemptCompletion> {
    let mut runtime = None;
    for (index, task) in snapshot.tasks.iter().enumerate() {
        let observer: Arc<dyn TaskOutputObserver> = Arc::new(DurableObserver {
            store: store.clone(),
            command: command.clone(),
            task_id: task.task_id.clone(),
        });
        let spec = worker_spec(command, snapshot, task, workspace_root)?;
        let result = execute_remote_worker_steps_with_output_and_cancellation(
            workspace_root,
            &spec,
            Some(Arc::clone(&observer)),
            cancellation,
        )
        .await?;
        let task_runtime = runtime_metadata(&result);
        if !result.success {
            return Ok(failed_with_exit_code(
                anyhow::anyhow!("task exited {:?}", result.exit_code),
                result.exit_code,
            )
            .with_runtime(task_runtime));
        }
        runtime = task_runtime;
        if index + 1 == snapshot.tasks.len()
            && matches!(
                store.begin_output_commit(command)?,
                super::super::scheduler::ResultAcceptance::Stale
            )
        {
            bail!("local attempt output fence is stale");
        }
        super::outputs::persist(store, command, task, workspace_root)?;
    }
    Ok(AttemptCompletion::Succeeded {
        terminal_digest: digest(b"succeeded"),
    }
    .with_runtime(runtime))
}

fn runtime_metadata(result: &RemoteWorkerExecutionOutcome) -> Option<AttemptRuntimeMetadata> {
    Some(AttemptRuntimeMetadata {
        kind: result.runtime_kind.clone()?,
        engine: result.runtime_engine.clone()?,
    })
}

fn worker_spec(
    command: &DispatchCommand,
    snapshot: &LocalExecutionSnapshot,
    task: &ResolvedTaskUnit,
    workspace_root: &Path,
) -> Result<RemoteWorkerExecutionSpec> {
    let mut environment =
        target_environment(command, task, &snapshot.attempt_root, workspace_root)?;
    for name in &task.pass_env_names {
        let value = snapshot
            .environment
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("passed environment `{name}` is missing"))?;
        environment.insert(name.clone(), value.clone());
    }
    Ok(RemoteWorkerExecutionSpec {
        task_label: parse_label(&task.task_id, "//")?,
        task_run_id: format!("{}/{}/{}", command.run_id, command.job_id, task.task_id),
        attempt: command.authored_attempt,
        steps: task.steps.iter().map(step).collect(),
        base_environment: environment,
        clear_environment: true,
        timeout_s: task.timeout_s,
        runtime: super::super::task_runtime::runner_runtime(
            task.runtime.as_ref(),
            &snapshot.attempt_root,
        )?,
        node_id: command.node_id.clone(),
        container_user: super::super::task_runtime::daemon_container_user(),
        image_cache: None,
        container_identity: None,
    })
}

fn target_environment(
    command: &DispatchCommand,
    task: &ResolvedTaskUnit,
    attempt_root: &Path,
    workspace_root: &Path,
) -> Result<BTreeMap<String, String>> {
    let home = attempt_root.join("home");
    let temporary = attempt_root.join("tmp");
    std::fs::create_dir_all(&home)?;
    std::fs::create_dir_all(&temporary)?;
    let mut result = BTreeMap::from([
        ("HOME".into(), home.display().to_string()),
        ("TMPDIR".into(), temporary.display().to_string()),
        ("TMP".into(), temporary.display().to_string()),
        ("TEMP".into(), temporary.display().to_string()),
        ("TAK_RUN_ID".into(), command.run_id.clone()),
        ("TAK_JOB_ID".into(), command.job_id.clone()),
        ("TAK_TASK_ID".into(), task.task_id.clone()),
        ("TAK_NODE_ID".into(), command.node_id.clone()),
        ("TAK_ATTEMPT".into(), command.authored_attempt.to_string()),
        ("TAK_WORKSPACE".into(), workspace_root.display().to_string()),
    ]);
    super::super::task_runtime::insert_host_path_for_native_runtime(
        &mut result,
        task.runtime.as_ref(),
    );
    result.insert(
        "TAK_DISPATCH_GENERATION".into(),
        command.dispatch_generation.to_string(),
    );
    Ok(result)
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

pub(super) fn failed(error: anyhow::Error) -> AttemptCompletion {
    failed_with_exit_code(error, None)
}

fn failed_with_exit_code(error: anyhow::Error, exit_code: Option<i32>) -> AttemptCompletion {
    AttemptCompletion::Failed {
        terminal_digest: digest(format!("failed:{error:#}").as_bytes()),
        exit_code,
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct DurableObserver {
    store: RunStore,
    command: DispatchCommand,
    task_id: String,
}

impl TaskOutputObserver for DurableObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> Result<()> {
        let stream = match chunk.stream {
            OutputStream::Stdout => AttemptOutputStream::Stdout,
            OutputStream::Stderr => AttemptOutputStream::Stderr,
        };
        let acceptance =
            self.store
                .append_attempt_output(&self.command, &self.task_id, stream, &chunk.bytes)?;
        if matches!(acceptance, super::super::scheduler::ResultAcceptance::Stale) {
            bail!("local attempt output fence is stale");
        }
        Ok(())
    }
}
