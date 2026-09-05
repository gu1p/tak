use std::collections::BTreeSet;

use tak_core::v2::ResolvedRun;
use tak_proto::local_daemon::v2::RunDetails;

use super::event_text::safe_terminal_field;
use super::{DashboardJob, DashboardJobSeed, DashboardSeed, JobActivity};

impl DashboardSeed {
    pub(in crate::cli) fn from_resolved(run_id: &str, run: &ResolvedRun) -> Self {
        let blocked = run
            .job_edges
            .iter()
            .map(|edge| edge.dependent_job_id.as_str())
            .collect::<BTreeSet<_>>();
        let jobs = run
            .jobs
            .iter()
            .map(|job| DashboardJobSeed {
                job_id: job.job_id.clone(),
                task_ids: job.task_ids.clone(),
                state: initial_state(blocked.contains(job.job_id.as_str())).into(),
                node_id: None,
                candidate_node_ids: job
                    .placement_candidates
                    .iter()
                    .map(|candidate| candidate.node_id.clone())
                    .collect(),
                queue: job.queue.clone(),
                attempt: 0,
                cache: None,
            })
            .collect();
        Self {
            run_id: run_id.into(),
            lifecycle: "submitted".into(),
            max_parallel_jobs: run.options.max_parallel_jobs.get(),
            jobs,
        }
    }

    pub(in crate::cli) fn from_details(details: &RunDetails) -> Self {
        Self {
            run_id: details.summary.run_id.clone(),
            lifecycle: details.summary.state.as_str().into(),
            max_parallel_jobs: details.max_parallel_jobs,
            jobs: details
                .jobs
                .iter()
                .map(|job| DashboardJobSeed {
                    job_id: job.job_id.clone(),
                    task_ids: job.task_ids.clone(),
                    state: job.state.clone(),
                    node_id: job.node_id.clone(),
                    candidate_node_ids: job.placement_candidate_node_ids.clone(),
                    queue: job.queue.clone(),
                    attempt: job.attempt,
                    cache: job.cache.clone(),
                })
                .collect(),
        }
    }
}

impl From<DashboardJobSeed> for DashboardJob {
    fn from(seed: DashboardJobSeed) -> Self {
        let activity = JobActivity::from_state(&seed.state);
        let attempt = if matches!(
            activity,
            JobActivity::Staging
                | JobActivity::Blocked
                | JobActivity::Ready
                | JobActivity::Retrying
        ) {
            0
        } else {
            seed.attempt
        };
        Self {
            task_ids: seed
                .task_ids
                .iter()
                .map(|task| safe_terminal_field(task))
                .collect(),
            activity,
            node_id: seed.node_id.as_deref().map(safe_terminal_field),
            attempt,
            cache: seed.cache.as_deref().map(safe_terminal_field),
            candidate_node_ids: seed
                .candidate_node_ids
                .iter()
                .map(|node| safe_terminal_field(node))
                .collect(),
            queue: seed.queue.as_deref().map(safe_terminal_field),
        }
    }
}

impl From<(&str, &ResolvedRun)> for DashboardSeed {
    fn from((run_id, run): (&str, &ResolvedRun)) -> Self {
        Self::from_resolved(run_id, run)
    }
}

impl From<&RunDetails> for DashboardSeed {
    fn from(details: &RunDetails) -> Self {
        Self::from_details(details)
    }
}

fn initial_state(blocked: bool) -> &'static str {
    if blocked { "blocked" } else { "staging" }
}
