use std::num::NonZeroU32;

use tak_core::v2::{
    JobEdge, PlacementCandidate, PlacementKind, PlacementPolicy, RemoteSelection, ResourceRequest,
};
use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::RunStore;

use super::{ARCHIVE, submission};

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

pub fn dependent_jobs(key: &str, keep_going: bool) -> tak_core::v2::RunSubmission {
    let mut result = independent_jobs(key, 3);
    result.run.options.max_parallel_jobs = NonZeroU32::MIN;
    result.run.options.keep_going = keep_going;
    result.run.tasks[1].dependencies = vec![result.run.tasks[0].task_id.clone()];
    result.run.job_edges = vec![JobEdge {
        dependency_job_id: result.run.jobs[0].job_id.clone(),
        dependent_job_id: result.run.jobs[1].job_id.clone(),
    }];
    result
}

pub fn commit(store: &RunStore, request: &tak_core::v2::RunSubmission, owner: &str) -> String {
    let accepted = store.submit(request, owner).unwrap();
    if matches!(
        accepted.workspace,
        WorkspaceDisposition::UploadRequired { .. }
    ) {
        store
            .upload_workspace(
                &accepted.run_id,
                &request.run.workspace.manifest.fingerprint,
                ARCHIVE.len() as u64,
                0,
                &ARCHIVE,
            )
            .unwrap();
    }
    store.commit(&accepted.run_id).unwrap();
    accepted.run_id
}
