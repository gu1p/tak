use std::num::NonZeroU32;

use tak_core::v2::{Affinity, Session, SessionReuse};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn task_dependencies_must_exactly_project_to_an_acyclic_job_graph() {
    let mut missing_edge = sample_run();
    let mut dependency = missing_edge.tasks[0].clone();
    dependency.task_id = "//:dep".into();
    dependency.job_id = "job-1".into();
    dependency.dependencies.clear();
    let mut dependency_job = missing_edge.jobs[0].clone();
    dependency_job.job_id = "job-1".into();
    dependency_job.task_ids = vec!["//:dep".into()];
    missing_edge.tasks[0].dependencies = vec!["//:dep".into()];
    missing_edge.tasks.push(dependency);
    missing_edge.jobs.push(dependency_job);
    assert!(missing_edge.validate().is_err());

    let mut fused_cycle = sample_run();
    fused_cycle.tasks[0].dependencies = vec!["//:check".into()];
    assert!(fused_cycle.validate().is_err());
}

#[test]
fn job_policy_must_be_the_conservative_projection_of_its_tasks() {
    let mut idempotency = sample_run();
    idempotency.jobs[0].idempotent = true;
    assert!(idempotency.validate().is_err());

    let mut environment = sample_run();
    environment.jobs[0].pass_env_names.clear();
    assert!(environment.validate().is_err());

    let mut affinity = sample_run();
    affinity.jobs[0].affinity = Some(Affinity::prefer_same_node("other").unwrap());
    assert!(affinity.validate().is_err());
}

#[test]
fn deserialized_shared_sessions_revalidate_their_hard_affinity() {
    let mut run = sample_run();
    run.jobs[0].session = Some(Session {
        id: "shared".into(),
        name: Some("shared".into()),
        reuse: SessionReuse::SharedWorkspace {
            max_parallel_tasks: NonZeroU32::MIN,
        },
        affinity: None,
        execution: None,
    });
    assert!(run.validate().is_err());
}
