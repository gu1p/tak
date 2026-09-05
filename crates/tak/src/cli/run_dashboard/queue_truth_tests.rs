use tak_proto::local_daemon::v2::RunEventKind;

use super::model::DashboardState;
use super::render_test_support::frame;
use super::test_support::{event, seed, state};

#[test]
fn nodes_label_candidate_queues_without_claiming_live_eligibility_or_rank() {
    let rendered = frame(&state(), 118);
    let queued = rendered
        .lines()
        .find(|line| line.contains("candidates:"))
        .expect("waiting task row");

    assert!(rendered.contains("1 candidate"), "{rendered}");
    assert!(!rendered.contains("ELIGIBLE QUEUE"), "{rendered}");
    assert!(queued.contains("//:lint"), "{queued}");
    assert!(
        queued.contains("candidates: worker-a, worker-b"),
        "{queued}"
    );
    assert!(
        !queued.contains("1. //:lint"),
        "fake scheduler rank: {queued}"
    );
}

#[test]
fn candidate_without_assigned_work_is_not_called_globally_idle() {
    let mut state = state();
    state
        .apply(&event(1, RunEventKind::Succeeded, "test", Some("worker-b")))
        .unwrap();
    let rendered = frame(&state, 118);
    let worker = rendered
        .split_once("worker-b")
        .and_then(|(_, rest)| rest.split_once("TASKS"))
        .map(|(worker, _)| worker)
        .expect("candidate node lane");

    assert!(worker.contains("0 active tasks"), "{worker}");
    assert!(!worker.contains("idle"), "{worker}");
}

#[test]
fn assigned_jobs_are_not_reported_as_global_unassigned_waiting_work() {
    let mut state = state();
    state.jobs.get_mut("lint").unwrap().node_id = Some("worker-a".into());

    assert!(!state.scheduler_queue().contains(&"lint"));
}

#[test]
fn precommit_staging_enters_the_queue_only_after_the_daemon_queued_event() {
    let mut seed = seed();
    seed.lifecycle = "submitted".into();
    seed.jobs[2].state = "staging".into();
    let mut state = DashboardState::new(seed);
    assert!(!state.scheduler_queue().contains(&"lint"));

    let mut queued = event(1, RunEventKind::Queued, "lint", None);
    queued.job_id = None;
    queued.task_ids.clear();
    state.apply(&queued).unwrap();

    assert!(state.scheduler_queue().contains(&"lint"));
}

#[test]
fn unknown_attach_parallelism_never_invents_a_one_job_limit() {
    let mut seed = seed();
    seed.max_parallel_jobs = 0;
    let rendered = frame(&DashboardState::new(seed), 118);

    assert!(rendered.contains("2 active"), "{rendered}");
    assert!(
        !rendered.contains("/1 active") && !rendered.contains("/0 active"),
        "{rendered}"
    );
}
