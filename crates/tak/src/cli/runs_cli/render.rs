use tak_proto::local_daemon::v2::{RunDetails, RunEvent, RunEventKind, RunSummary};

pub(super) fn list(runs: &[RunSummary]) {
    for run in runs {
        println!(
            "{} {} jobs={}/{} targets={}",
            run.run_id,
            run.state.as_str(),
            run.terminal_jobs,
            run.total_jobs,
            run.targets.join(",")
        );
    }
}

pub(super) fn details(run: &RunDetails) {
    list(std::slice::from_ref(&run.summary));
    for job in &run.jobs {
        println!(
            "{} {} tasks={} node={} attempt={} cache={}",
            job.job_id,
            job.state,
            job.task_ids.join(","),
            job.node_id.as_deref().unwrap_or("-"),
            job.attempt,
            job.cache.as_deref().unwrap_or("-")
        );
    }
}

pub(super) fn events(events: &[RunEvent]) {
    for event in events {
        println!(
            "{} tasks={} node={} {}",
            event_kind(event.kind),
            event.task_ids.join(","),
            event.node_id.as_deref().unwrap_or("-"),
            event.message
        );
    }
}

fn event_kind(kind: RunEventKind) -> &'static str {
    match kind {
        RunEventKind::Submitted => "submitted",
        RunEventKind::WorkspaceUploading => "workspace_uploading",
        RunEventKind::Queued => "queued",
        RunEventKind::Transferring => "transferring",
        RunEventKind::Running => "running",
        RunEventKind::Retrying => "retrying",
        RunEventKind::OutputCommitting => "output_committing",
        RunEventKind::Cancelling => "cancelling",
        RunEventKind::Succeeded => "succeeded",
        RunEventKind::Failed => "failed",
        RunEventKind::Cancelled => "cancelled",
        RunEventKind::Skipped => "skipped",
    }
}
