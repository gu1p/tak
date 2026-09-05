use super::model::{DashboardJobSeed, DashboardSeed, DashboardState};

pub(super) fn state() -> DashboardState {
    DashboardState::new(seed())
}

pub(super) fn seed() -> DashboardSeed {
    DashboardSeed {
        run_id: "run-high-end".into(),
        lifecycle: "running".into(),
        max_parallel_jobs: 3,
        jobs: vec![
            job(
                "build",
                "//:build",
                "running",
                Some("worker-a"),
                &["worker-a", "worker-b"],
            ),
            job(
                "test",
                "//:test",
                "transferring",
                Some("worker-b"),
                &["worker-a", "worker-b"],
            ),
            job("lint", "//:lint", "ready", None, &["worker-a", "worker-b"]),
        ],
    }
}

fn job(
    job_id: &str,
    task_id: &str,
    state: &str,
    node_id: Option<&str>,
    candidates: &[&str],
) -> DashboardJobSeed {
    DashboardJobSeed {
        job_id: job_id.into(),
        task_ids: vec![task_id.into()],
        state: state.into(),
        node_id: node_id.map(str::to_owned),
        candidate_node_ids: candidates.iter().map(|node| (*node).into()).collect(),
        queue: Some("builds".into()),
        attempt: u32::from(node_id.is_some()),
        cache: node_id.map(|_| "miss".into()),
    }
}

pub(super) fn event(
    seq: u64,
    kind: tak_proto::local_daemon::v2::RunEventKind,
    job: &str,
    node: Option<&str>,
) -> tak_proto::local_daemon::v2::RunEvent {
    tak_proto::local_daemon::v2::RunEvent {
        seq,
        kind,
        job_id: Some(job.into()),
        task_ids: vec![format!("//:{job}")],
        node_id: node.map(str::to_owned),
        authored_attempt: None,
        message: String::new(),
        chunk_base64: None,
        exit_code: None,
    }
}

pub(super) fn node_frame(state: &DashboardState, width: u16, height: u16) -> String {
    use super::navigation::{DashboardNavigation, NavigationAction};
    let mut navigation = DashboardNavigation::default();
    navigation.apply(NavigationAction::PreviousPanel);
    super::render_test_support::frame_at_size_with_navigation(state, &navigation, width, height)
}
