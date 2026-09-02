use std::process::Command;
use std::time::{Duration, Instant};

use takd::RunStore;

use crate::support::v2_run::submission;

const CHILD_ENV: &str = "TAKD_CONNECTION_LIFETIME_CHILD";
const TEST_FILTER: &str = "cloned_attachment_and_summary_connections_finish";

#[test]
fn cloned_attachment_and_summary_connections_finish_without_sqlite_deadlock() {
    if std::env::var_os(CHILD_ENV).is_some() {
        exercise_concurrent_connections();
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap())
        .arg(TEST_FILTER)
        .arg("--nocapture")
        .env(CHILD_ENV, "1")
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "connection race child failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            child.wait().unwrap();
            panic!("cloned RunStore connection race exceeded 15 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn exercise_concurrent_connections() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let store = RunStore::with_db_path(temp.path().join("run.sqlite")).unwrap();
    let run_id = store
        .submit(&submission("connection-lifetime", "secret"), "alice")
        .unwrap()
        .run_id;
    let attachments = store.clone();
    let summaries = store.clone();
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let attachment_run = run_id.clone();
            let attachment_task = tokio::spawn(async move {
                for _ in 0..2_000 {
                    attachments
                        .attachment_snapshot(&attachment_run, 0)
                        .unwrap()
                        .unwrap();
                    tokio::task::yield_now().await;
                }
            });
            let summary_task = tokio::spawn(async move {
                for _ in 0..2_000 {
                    summaries.summary(&run_id).unwrap().unwrap();
                    tokio::task::yield_now().await;
                }
            });
            attachment_task.await.unwrap();
            summary_task.await.unwrap();
        });
}
