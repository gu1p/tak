use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use base64::Engine;
use tak_core::v2::Step;
use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::restartable_local_daemon::RestartableLocalDaemon;
use crate::support::v2_run;

#[test]
fn a_local_attempt_survives_a_real_takd_process_restart() {
    let mut daemon = RestartableLocalDaemon::start();
    let started = daemon.scratch_path("started");
    let release = daemon.scratch_path("release");
    let completed = daemon.scratch_path("completed");
    let store = RunStore::with_db_path(daemon.state_root().join("takd.sqlite")).unwrap();
    let mut request = v2_run::submission("restart-local", "secret");
    request.run.tasks[0].idempotent = false;
    request.run.tasks[0].steps = vec![blocking_step(&started, &release, &completed)];
    request.run.jobs[0].idempotent = false;
    request.run.jobs[0].retry.max_attempts = NonZeroU32::new(2).unwrap();
    let run_id = v2_run::scheduler::commit(&store, &request, "restart-owner");
    wait_for(|| started.exists());
    wait_for(|| output(&store, &run_id) == b"before");

    daemon.crash_and_restart();
    std::fs::write(&release, b"release").unwrap();
    wait_for(|| {
        store
            .summary(&run_id)
            .unwrap()
            .is_some_and(|run| run.state.is_terminal())
    });

    let summary = store.summary(&run_id).unwrap().unwrap();
    let details = store.get_run(&run_id).unwrap().unwrap();
    assert_eq!(summary.state, RunLifecycleState::Succeeded);
    assert_eq!(details.jobs[0].attempt, 1);
    assert_eq!(std::fs::read(&completed).unwrap(), b"completed");
    assert_eq!(output(&store, &run_id), b"beforeafter");
}

pub(super) fn blocking_step(
    started: &std::path::Path,
    release: &std::path::Path,
    done: &std::path::Path,
) -> Step {
    let script = "printf before; printf started > \"$1\"; i=0; while [ ! -f \"$2\" ] && [ \"$i\" -lt 500 ]; do /bin/sleep 0.02; i=$((i + 1)); done; test -f \"$2\" || exit 9; printf completed > \"$3\"; printf after";
    Step::Cmd {
        argv: ["/bin/sh", "-c", script, "restart-test"]
            .into_iter()
            .map(str::to_owned)
            .chain([started, release, done].into_iter().map(|path| path.display().to_string()))
            .collect(),
        cwd: None,
        env: BTreeMap::new(),
    }
}

pub(super) fn output(store: &RunStore, run_id: &str) -> Vec<u8> {
    store
        .events_after(run_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| event.kind == RunEventKind::Stdout)
        .flat_map(|event| {
            base64::engine::general_purpose::STANDARD
                .decode(event.chunk_base64.unwrap())
                .unwrap()
        })
        .collect()
}

pub(super) fn wait_for(predicate: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !predicate() {
        assert!(Instant::now() < deadline, "condition did not become true");
        std::thread::sleep(Duration::from_millis(20));
    }
}
