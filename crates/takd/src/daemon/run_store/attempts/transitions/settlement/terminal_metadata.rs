use anyhow::Result;
use rusqlite::Transaction;
use sha2::{Digest, Sha256};
use tak_core::v2::{PlacementKind, ResolvedJob, ResolvedRun, SessionReuse, TaskRuntime};

use crate::daemon::scheduler::{AttemptRuntimeMetadata, DispatchCommand};

pub(super) fn render(
    transaction: &Transaction<'_>,
    command: &DispatchCommand,
    job: &ResolvedJob,
    outcome: &str,
    exit_code: Option<i32>,
    actual_runtime: Option<&AttemptRuntimeMetadata>,
) -> Result<String> {
    let run = resolved_run(transaction, &command.run_id)?;
    let candidate = job
        .placement_candidates
        .iter()
        .find(|candidate| candidate.node_id == command.node_id)
        .ok_or_else(|| anyhow::anyhow!("settled node is not a placement candidate"))?;
    let placement = match candidate.kind {
        PlacementKind::Local => "local",
        PlacementKind::Remote => "remote",
    };
    let remote_node = match candidate.kind {
        PlacementKind::Local => "none",
        PlacementKind::Remote => command.node_id.as_str(),
    };
    let (runtime, engine) = runtime(&run, job, actual_runtime);
    let (session, reuse) = session(job);
    Ok(format!(
        "{outcome} (task_run_id={}, attempts={}, exit_code={}, placement={placement}, remote_node={remote_node}, transport={}, reason={}, context_hash={}, runtime={runtime}, runtime_engine={engine}, session={session}, reuse={reuse})",
        command.job_id,
        command.authored_attempt,
        exit_code.map_or_else(|| "none".into(), |code| code.to_string()),
        command.transport.as_deref().unwrap_or("none"),
        candidate.reason,
        context_hash(job),
    ))
}

fn resolved_run(transaction: &Transaction<'_>, run_id: &str) -> Result<ResolvedRun> {
    let encoded = transaction.query_row(
        "SELECT resolved_json FROM runs WHERE run_id=?1",
        [run_id],
        |row| row.get::<_, String>(0),
    )?;
    Ok(serde_json::from_str(&encoded)?)
}

fn runtime<'a>(
    run: &ResolvedRun,
    job: &ResolvedJob,
    actual: Option<&'a AttemptRuntimeMetadata>,
) -> (&'a str, &'a str) {
    if let Some(actual) = actual {
        return (&actual.kind, &actual.engine);
    }
    let containerized = run.tasks.iter().any(|task| {
        task.job_id == job.job_id && matches!(task.runtime, Some(TaskRuntime::Container { .. }))
    });
    if containerized {
        ("containerized", "none")
    } else {
        ("none", "none")
    }
}

fn session(job: &ResolvedJob) -> (&str, &'static str) {
    let Some(session) = job.session.as_ref() else {
        return ("none", "none");
    };
    let reuse = match session.reuse {
        SessionReuse::Workspace => "workspace",
        SessionReuse::Paths { .. } => "share_paths",
        SessionReuse::SharedWorkspace { .. } => "share_workspace",
        SessionReuse::Container => "container",
    };
    (session.name.as_deref().unwrap_or(&session.id), reuse)
}

fn context_hash(job: &ResolvedJob) -> String {
    let mut hash = Sha256::new();
    for path in &job.context_manifest.paths {
        hash.update((path.len() as u64).to_be_bytes());
        hash.update(path.as_bytes());
    }
    format!("{:x}", hash.finalize())
}
