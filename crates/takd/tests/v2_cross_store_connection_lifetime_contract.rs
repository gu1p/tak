use std::process::Command;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use tak_proto::worker_v2::WorkerOutputStream;
use takd::{RunStore, SubmitAttemptStore};

use crate::support::v2_worker::dispatch;

const CHILD_ENV: &str = "TAKD_CROSS_STORE_CONNECTION_CHILD";
const TEST_FILTER: &str = "cross_store_connection_lifetimes_finish";
const ITERATIONS: usize = 2_000;

#[test]
fn cross_store_connection_lifetimes_finish_without_sqlite_deadlock() {
    if std::env::var_os(CHILD_ENV).is_some() {
        exercise_cross_store_connections();
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg(TEST_FILTER)
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "cross-store race child failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            child.wait().unwrap();
            panic!("cross-store SQLite connection race exceeded 120 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn exercise_cross_store_connections() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let runs = RunStore::with_db_path(temp.path().join("runs.sqlite")).unwrap();
    let attempts = SubmitAttemptStore::with_db_path(temp.path().join("attempts.sqlite")).unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let mut threads = Vec::new();
    for index in 0..2 {
        let mut request = dispatch(1, 1, &format!("cross-store-fence-{index}"));
        request.identity.run_id = format!("run-{index}");
        attempts.register_worker_v2_attempt(&request).unwrap();
        attempts.mark_worker_v2_running(&request.identity).unwrap();
        let store = attempts.clone();
        let identity = request.identity;
        let start = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            for _ in 0..ITERATIONS {
                start.wait();
                store
                    .append_worker_v2_event(
                        &identity,
                        "//:check",
                        WorkerOutputStream::Stdout,
                        b"x",
                    )
                    .unwrap();
            }
        }));
    }
    for _ in 0..2 {
        let store = runs.clone();
        let start = Arc::clone(&barrier);
        threads.push(std::thread::spawn(move || {
            for _ in 0..ITERATIONS {
                start.wait();
                assert!(store.pending_dispatches().unwrap().is_empty());
            }
        }));
    }
    for thread in threads {
        thread.join().unwrap();
    }
}
