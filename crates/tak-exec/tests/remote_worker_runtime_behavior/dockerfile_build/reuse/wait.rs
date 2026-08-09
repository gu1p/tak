use std::time::Duration;

use crate::support::FakeDockerDaemon;

pub(super) async fn for_build_count(daemon: &FakeDockerDaemon, expected: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if daemon.build_records().len() >= expected {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("build request should reach fake docker daemon");
}
