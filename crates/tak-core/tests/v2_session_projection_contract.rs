use tak_core::v2::{Affinity, Session, SessionReuse};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn resolved_sessions_must_project_their_effective_affinity() {
    let hard = Affinity::require_same_node("shared").unwrap();
    let session = Session::new(
        "shared",
        SessionReuse::shared_workspace(1).unwrap(),
        Some(hard),
    )
    .unwrap();
    let mut missing = sample_run();
    missing.jobs[0].session = Some(session);
    assert!(missing.validate().is_err());
}

#[test]
fn one_session_id_cannot_have_conflicting_resolved_definitions() {
    let hard = Affinity::require_same_node("shared").unwrap();
    let mut run = sample_run();
    run.tasks[0].affinity = Some(hard.clone());
    run.jobs[0].affinity = Some(hard.clone());
    run.jobs[0].session = Some(shared_session(1, hard.clone()));
    let mut task = run.tasks[0].clone();
    task.task_id = "//:other".into();
    task.job_id = "job-1".into();
    let mut job = run.jobs[0].clone();
    job.job_id = task.job_id.clone();
    job.task_ids = vec![task.task_id.clone()];
    job.session = Some(shared_session(2, hard));
    run.targets.push(task.task_id.clone());
    run.tasks.push(task);
    run.jobs.push(job);
    assert!(run.validate().is_err());
}

fn shared_session(cap: u32, affinity: Affinity) -> Session {
    Session::new(
        "shared",
        SessionReuse::shared_workspace(cap).unwrap(),
        Some(affinity),
    )
    .unwrap()
}
