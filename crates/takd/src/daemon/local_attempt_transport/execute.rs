use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, bail};
use sha2::{Digest, Sha256};
use tak_core::label::parse_label;
use tak_core::model::StepDef;
use tak_core::v2::{ResolvedTaskUnit, Step};
use tak_runner::{
    OutputStream, RemoteWorkerExecutionSpec, TaskOutputChunk, TaskOutputObserver,
    execute_remote_worker_steps_with_output_and_cancellation,
};

use super::super::run_store::RunStore;
use super::super::run_store::execution::LocalExecutionSnapshot;
use super::super::scheduler::{AttemptCompletion, AttemptOutputStream, DispatchCommand};
use super::{ActiveAttempt, workspace};

pub(super) fn spawn(
    store: RunStore,
    command: DispatchCommand,
    snapshot: LocalExecutionSnapshot,
    workspace_root: PathBuf,
    active: Arc<ActiveAttempt>,
) {
    tokio::spawn(async move {
        let completion = run(&store, &command, &snapshot, &workspace_root, &active)
            .await
            .unwrap_or_else(failed);
        loop {
            match workspace::write_completion(&snapshot.attempt_root, &completion) {
                Ok(()) => break,
                Err(error) => {
                    tracing::error!("persist local attempt terminal record: {error:#}");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
        *active.completion.lock().expect("active attempt lock") = Some(completion);
        active.completed.notify_waiters();
    });
}

async fn run(
    store: &RunStore,
    command: &DispatchCommand,
    snapshot: &LocalExecutionSnapshot,
    workspace_root: &Path,
    active: &ActiveAttempt,
) -> Result<AttemptCompletion> {
    for task in &snapshot.tasks {
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
            &active.cancellation,
        )
        .await?;
        if !result.success {
            return Ok(failed(anyhow::anyhow!(
                "task exited {:?}",
                result.exit_code
            )));
        }
    }
    Ok(AttemptCompletion::Succeeded {
        terminal_digest: digest(b"succeeded"),
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
        timeout_s: None,
        runtime: None,
        node_id: command.node_id.clone(),
        container_user: None,
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
        (
            "PATH".into(),
            std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".into()),
        ),
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

fn failed(error: anyhow::Error) -> AttemptCompletion {
    AttemptCompletion::Failed {
        terminal_digest: digest(format!("failed:{error:#}").as_bytes()),
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
