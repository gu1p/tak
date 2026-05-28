#![cfg(test)]

use std::time::Duration;

use super::protocol_submit::remote_submit_timeout;

#[path = "protocol_submit_tests/daemon_submit.rs"]
mod daemon_submit;

#[test]
fn direct_remote_submit_budget_allows_loaded_test_agents_to_acknowledge() {
    assert!(remote_submit_timeout() >= Duration::from_secs(30));
}
