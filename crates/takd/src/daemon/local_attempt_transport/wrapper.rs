use std::path::Path;
use std::time::Duration;

use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use tak_core::v2::OutputMergeError;
use tak_runner::RunCancellation;

use super::{durable_state::AttemptOwner, execute, launcher, workspace};
use crate::daemon::run_store::RunStore;
use crate::daemon::scheduler::DispatchCommand;

pub async fn run_local_attempt_subprocess(request_path: &Path) -> Result<()> {
    let request = launcher::read_request(request_path)?;
    let store = RunStore::with_db_path(request.db_path)?;
    let root = store.attempt_root(&request.command);
    ensure!(
        std::fs::canonicalize(request_path)? == std::fs::canonicalize(root.join("request.json"))?,
        "local attempt request path does not match its dispatch identity"
    );
    let Some(_owner) = AttemptOwner::try_acquire(&root)? else {
        return Ok(());
    };
    if workspace::read_completion(&root)?.is_some() {
        return Ok(());
    }
    let snapshot = match store.local_execution_snapshot(&request.command) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let Some(conflict) = error.downcast_ref::<OutputMergeError>() else {
                return Err(error);
            };
            let message = format!("declared output preparation failed: {conflict}");
            let digest = format!("{:x}", Sha256::digest(message.as_bytes()));
            store.fail_attempt_permanently(&request.command, &digest, &message)?;
            return Ok(());
        }
    };
    let prepared = tokio::task::spawn_blocking(move || workspace::prepare(snapshot)).await??;
    let workspace::Preparation::Execute {
        snapshot,
        workspace_root,
    } = prepared
    else {
        return Ok(());
    };
    if !store.local_attempt_is_current(&request.command)? {
        return Ok(());
    }
    workspace::mark_started(&root)?;
    let cancellation = RunCancellation::new();
    let watcher =
        spawn_cancellation_watcher(store.clone(), request.command.clone(), cancellation.clone());
    let completion = execute::run(
        &store,
        &request.command,
        &snapshot,
        &workspace_root,
        &cancellation,
    )
    .await
    .unwrap_or_else(execute::failed);
    watcher.abort();
    persist_completion(&root, &completion).await
}

fn spawn_cancellation_watcher(
    store: RunStore,
    command: DispatchCommand,
    cancellation: RunCancellation,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            match store.local_attempt_is_current(&command) {
                Ok(true) => tokio::time::sleep(Duration::from_millis(20)).await,
                Ok(false) => {
                    cancellation.cancel();
                    return;
                }
                Err(error) => {
                    tracing::debug!("poll durable local cancellation: {error:#}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    })
}

async fn persist_completion(
    root: &Path,
    completion: &crate::daemon::scheduler::AttemptCompletion,
) -> Result<()> {
    loop {
        match workspace::write_completion(root, completion) {
            Ok(()) => return Ok(()),
            Err(error) => {
                tracing::error!("persist local attempt terminal record: {error:#}");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
