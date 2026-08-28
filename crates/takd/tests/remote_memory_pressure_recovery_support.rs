use std::{collections::BTreeMap, time::Duration};

use tokio::time::sleep;

use crate::support::fake_docker_daemon::{DockerOperation, FakeDockerDaemon};

pub(super) fn takd_labels(submit_key: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("tak.owner".to_string(), "takd".to_string()),
        ("tak.node_id".to_string(), "builder-a".to_string()),
        ("tak.submit_key".to_string(), submit_key.to_string()),
    ])
}

pub(super) async fn wait_for_unpause_attempts(daemon: &FakeDockerDaemon, expected: usize) {
    for _ in 0..250 {
        let attempts = daemon
            .operations()
            .iter()
            .filter(|operation| matches!(operation, DockerOperation::UnpauseAttempted(_)))
            .count();
        if attempts >= expected {
            return;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for {expected} unpause attempts");
}
