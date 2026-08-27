use std::time::{Duration, Instant};

use crate::support::fake_docker_daemon::{CreateRecord, FakeDockerDaemon};

pub(crate) fn wait_for_task_creates(
    daemon: &FakeDockerDaemon,
    task_run_id: &str,
    expected: usize,
) -> Vec<CreateRecord> {
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let records = daemon
            .create_records()
            .into_iter()
            .filter(|record| {
                record.labels.get("tak.task_run_id").map(String::as_str) == Some(task_run_id)
            })
            .collect::<Vec<_>>();
        if records.len() >= expected {
            return records;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {task_run_id} containers; all creates: {:?}",
            daemon.create_records()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
