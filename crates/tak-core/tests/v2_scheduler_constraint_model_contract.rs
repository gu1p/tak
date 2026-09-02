use std::num::{NonZeroU32, NonZeroU64};

use tak_core::v2::{
    Affinity, HoldMode, LimiterClaim, LimiterDefinition, QueueDefinition, QueueDiscipline,
};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn deserialized_affinity_groups_and_claims_are_admission_validated() {
    let mut affinity = sample_run();
    let invalid = Affinity::RequireSameNode {
        group: "   ".into(),
    };
    affinity.tasks[0].affinity = Some(invalid.clone());
    affinity.jobs[0].affinity = Some(invalid);
    assert!(affinity.validate().is_err());

    let mut duplicate = limited_run();
    let repeated_claim = duplicate.jobs[0].limiter_claims[0].clone();
    duplicate.jobs[0].limiter_claims.push(repeated_claim);
    assert!(duplicate.validate().is_err());

    let mut excessive = limited_run();
    excessive.jobs[0].limiter_claims[0].amount_millis = NonZeroU64::new(1_001).unwrap();
    assert!(excessive.validate().is_err());

    let mut partial_lock = limited_run();
    partial_lock.limiter_definitions = vec![LimiterDefinition::Lock {
        name: "cpu".into(),
        scope: tak_core::v2::DefinitionScope::Project,
        scope_key: None,
        hold: HoldMode::During,
    }];
    partial_lock.jobs[0].limiter_claims[0].amount_millis = NonZeroU64::new(500).unwrap();
    assert!(partial_lock.validate().is_err());
}

#[test]
fn a_hard_affinity_group_requires_a_common_candidate() {
    let mut run = sample_run();
    let affinity = Affinity::RequireSameNode {
        group: "build".into(),
    };
    run.tasks[0].affinity = Some(affinity.clone());
    run.jobs[0].affinity = Some(affinity.clone());
    let mut task = run.tasks[0].clone();
    task.task_id = "//:other".into();
    task.job_id = "job-1".into();
    let mut job = run.jobs[0].clone();
    job.job_id = task.job_id.clone();
    job.task_ids = vec![task.task_id.clone()];
    job.placement_candidates[0].node_id = "other".into();
    run.targets.push(task.task_id.clone());
    run.tasks.push(task);
    run.jobs.push(job);
    assert!(run.validate().is_err());
}

#[test]
fn priority_queues_accept_concrete_job_slots_and_priority() {
    let mut run = sample_run();
    run.queue_definitions = vec![QueueDefinition {
        name: "build".into(),
        scope: tak_core::v2::DefinitionScope::Project,
        scope_key: None,
        max_parallel_tasks: NonZeroU32::new(2).unwrap(),
        discipline: QueueDiscipline::Priority,
    }];
    run.jobs[0].queue = Some("build".into());
    run.jobs[0].queue_slots = NonZeroU32::new(2).unwrap();
    run.jobs[0].queue_priority = 100;
    assert!(run.validate().is_ok());

    run.jobs[0].queue_slots = NonZeroU32::new(3).unwrap();
    assert!(run.validate().is_err());
}

fn limited_run() -> tak_core::v2::ResolvedRun {
    let mut run = sample_run();
    run.limiter_definitions.push(LimiterDefinition::Resource {
        name: "cpu".into(),
        scope: tak_core::v2::DefinitionScope::Project,
        scope_key: None,
        capacity_millis: NonZeroU64::new(1_000).unwrap(),
        unit: None,
        hold: HoldMode::During,
    });
    run.jobs[0].limiter_claims.push(LimiterClaim {
        name: "cpu".into(),
        amount_millis: NonZeroU64::new(1_000).unwrap(),
    });
    run
}
