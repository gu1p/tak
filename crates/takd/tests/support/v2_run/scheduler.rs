use std::num::NonZeroU32;

use tak_core::v2::{
    PlacementCandidate, PlacementKind, PlacementPolicy, RemoteSelection, ResourceRequest,
};

use super::submission;

pub fn independent_jobs(key: &str, count: usize) -> tak_core::v2::RunSubmission {
    let mut result = submission(key, "secret");
    let task_template = result.run.tasks[0].clone();
    let job_template = result.run.jobs[0].clone();
    result.run.tasks.clear();
    result.run.jobs.clear();
    result.run.targets.clear();
    result.run.options.max_parallel_jobs = NonZeroU32::new(count as u32).unwrap();
    for index in 0..count {
        let task_id = format!("//:job-{index}");
        let job_id = format!("job-{index}");
        let mut task = task_template.clone();
        task.task_id.clone_from(&task_id);
        task.job_id.clone_from(&job_id);
        let mut job = job_template.clone();
        job.job_id = job_id;
        job.task_ids = vec![task_id.clone()];
        job.placement_policy = PlacementPolicy {
            policy_id: "workers".into(),
            selection: RemoteSelection::RoundRobin,
        };
        job.resources = ResourceRequest {
            cpu_millis: 0,
            memory_bytes: 0,
            execution_slots: NonZeroU32::MIN,
        };
        job.placement_candidates = ["worker-a", "worker-b"]
            .into_iter()
            .map(|node_id| PlacementCandidate {
                node_id: node_id.into(),
                kind: PlacementKind::Remote,
                transport: Some("direct".into()),
                reason: "healthy protocol-v2 worker".into(),
            })
            .collect();
        result.run.targets.push(task_id);
        result.run.tasks.push(task);
        result.run.jobs.push(job);
    }
    result = tak_core::v2::RunSubmission::new(
        result.idempotency_key,
        result.run,
        result.environment_values,
    )
    .unwrap();
    result
}
