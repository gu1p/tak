use std::num::NonZeroU32;

use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

use super::v2_local_attempt_restart_behavior::{blocking_step, output, wait_for};
use crate::support::restartable_local_daemon::RestartableLocalDaemon;
use crate::support::v2_run;

#[test]
fn cancellation_after_a_real_takd_restart_stops_the_durable_local_attempt() {
    let mut daemon = RestartableLocalDaemon::start();
    let started = daemon.scratch_path("cancel-started");
    let release = daemon.scratch_path("cancel-release");
    let completed = daemon.scratch_path("cancel-completed");
    let store = RunStore::with_db_path(daemon.state_root().join("takd.sqlite")).unwrap();
    let mut request = v2_run::submission("restart-cancel", "secret");
    request.run.tasks[0].idempotent = false;
    request.run.tasks[0].steps = vec![blocking_step(&started, &release, &completed)];
    request.run.jobs[0].idempotent = false;
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let run_id = v2_run::scheduler::commit(&store, &request, "restart-owner");
    wait_for(|| started.exists());
    wait_for(|| output(&store, &run_id) == b"before");

    daemon.crash_and_restart();
    assert_eq!(store.cancel(&run_id).unwrap(), RunLifecycleState::Cancelling);
    wait_for(|| {
        store
            .summary(&run_id)
            .unwrap()
            .is_some_and(|run| run.state.is_terminal())
    });

    let summary = store.summary(&run_id).unwrap().unwrap();
    let details = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(summary.state, RunLifecycleState::Cancelled);
    assert_eq!(details.jobs[0].attempt, 1);
    assert!(!completed.exists());
    assert_eq!(output(&store, &run_id), b"before");
}
