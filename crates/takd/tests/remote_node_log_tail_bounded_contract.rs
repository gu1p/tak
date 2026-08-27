#![cfg(unix)]

use std::{env, fs};

use crate::support::{self, bounded_log};
use takd::SubmitAttemptStore;

const CHILD_CASE: &str = "TAKD_TEST_BOUNDED_NODE_LOG_TAIL";
const TEST_NAME: &str =
    "remote_node_log_tail_bounded_contract::node_log_tail_reads_from_end_of_sparse_file";

#[test]
fn node_log_tail_reads_from_end_of_sparse_file() {
    if env::var_os(CHILD_CASE).is_some() {
        assert_node_log_tail();
        return;
    }
    let test_binary = env::current_exe().expect("current test binary");
    let output = bounded_log::command_with_data_limit(&test_binary)
        .env(CHILD_CASE, "1")
        .args([TEST_NAME, "--exact", "--nocapture"])
        .output()
        .expect("run memory-bounded node log test");
    assert!(
        output.status.success(),
        "node log tail read the whole sparse file:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_node_log_tail() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    bounded_log::write_sparse_log(
        &state_root.join("service.log"),
        "discard-me\nnode-last-one\nnode-last-two\n",
    );
    let context = support::remote_output::test_context().with_state_root(&state_root);
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let response = takd::daemon::remote::handle_remote_v1_request(
        &context,
        &store,
        "GET",
        "/v1/node/logs?lines=2",
        &[],
        None,
    )
    .expect("logs response");
    assert_eq!(response.status_code, 200);
    assert_eq!(
        String::from_utf8_lossy(&response.body),
        "node-last-one\nnode-last-two\n"
    );
}
