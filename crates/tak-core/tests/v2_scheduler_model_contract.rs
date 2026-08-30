use tak_core::v2::{PlacementKind, RemoteSelection};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn placement_policy_identity_and_concrete_candidates_are_canonical() {
    let mut invalid_policy = sample_run();
    invalid_policy.jobs[0].placement_policy.policy_id.clear();
    assert!(invalid_policy.validate().is_err());

    let mut duplicate_candidate = sample_run();
    let duplicate = duplicate_candidate.jobs[0].placement_candidates[0].clone();
    duplicate_candidate.jobs[0]
        .placement_candidates
        .push(duplicate);
    assert!(duplicate_candidate.validate().is_err());

    let mut contradictory_candidate = sample_run();
    contradictory_candidate.jobs[0].placement_candidates[0].kind = PlacementKind::Local;
    contradictory_candidate.jobs[0].placement_candidates[0].transport = Some("direct".into());
    assert!(contradictory_candidate.validate().is_err());
}

#[test]
fn one_run_policy_id_cannot_name_conflicting_selection_semantics() {
    let mut run = sample_run();
    let mut second_task = run.tasks[0].clone();
    second_task.task_id = "//:second".into();
    second_task.job_id = "job-1".into();
    let mut second_job = run.jobs[0].clone();
    second_job.job_id = "job-1".into();
    second_job.task_ids = vec![second_task.task_id.clone()];
    second_job.placement_policy.selection = RemoteSelection::Balanced;
    run.targets.push(second_task.task_id.clone());
    run.tasks.push(second_task);
    run.jobs.push(second_job);
    assert!(run.validate().is_err());
}
