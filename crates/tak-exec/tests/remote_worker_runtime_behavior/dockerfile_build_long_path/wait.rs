use std::time::Duration;

use crate::support::fake_docker_daemon::{BuildRecord, FakeDockerDaemon};

pub(super) async fn for_single_build(daemon: &FakeDockerDaemon) -> BuildRecord {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(build) = daemon.single_build() {
                break build;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("build request should reach fake docker daemon")
}
