use base64::{Engine as _, engine::general_purpose::STANDARD};
use tak_proto::local_daemon::v2::RunEventKind;

use super::model::{DashboardJobSeed, DashboardSeed, DashboardState};
use super::render_test_support::frame;
use super::test_support::event;

#[test]
fn metadata_fields_cannot_inject_controls_or_new_dashboard_lines() {
    let mut state = DashboardState::new(DashboardSeed {
        run_id: "run\x1b[2J\nforged-run".into(),
        lifecycle: "running".into(),
        max_parallel_jobs: 1,
        jobs: vec![DashboardJobSeed {
            job_id: "job-key".into(),
            task_ids: vec!["//:task\nforged-task\x07".into()],
            state: "running".into(),
            node_id: Some("node\rforged-node".into()),
            candidate_node_ids: vec!["candidate\tforged-candidate".into()],
            queue: Some("queue\nforged-queue".into()),
            attempt: 1,
            cache: Some("hit\x1b[2Jforged-cache".into()),
        }],
    });
    let mut fallback = event(
        1,
        RunEventKind::Stdout,
        "unused",
        Some("event-node\nforged"),
    );
    fallback.job_id = Some("job-fallback\nforged-job".into());
    fallback.task_ids.clear();
    fallback.chunk_base64 = Some(STANDARD.encode("safe output\n"));
    state.apply(&fallback).unwrap();

    let job = &state.jobs["job-key"];
    let fields = [
        state.run_id.as_str(),
        job.task_ids[0].as_str(),
        job.node_id.as_deref().unwrap(),
        job.candidate_node_ids[0].as_str(),
        job.queue.as_deref().unwrap(),
        job.cache.as_deref().unwrap(),
        state.logs[0].job.as_str(),
        state.logs[0].node.as_str(),
    ];
    assert!(
        fields
            .iter()
            .all(|field| field.chars().all(|character| !character.is_control())),
        "unsafe metadata fields: {fields:?}"
    );

    let rendered = format!(
        "{}\n{}",
        frame(&state, 118),
        super::terminal::final_summary(&state)
    );
    assert!(
        rendered
            .chars()
            .all(|character| character == '\n' || !character.is_control()),
        "unsafe rendered metadata: {rendered:?}"
    );
}
