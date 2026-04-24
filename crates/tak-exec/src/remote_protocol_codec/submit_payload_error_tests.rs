use super::submit_payload_test_support::{direct_target, task_with_steps_and_needs, workspace};
use super::*;

#[test]
fn build_remote_submit_payload_rejects_invalid_workspace_archive() {
    let err = build_remote_submit_payload(
        &direct_target(None),
        "task-run-1",
        1,
        &task_with_steps_and_needs(),
        &workspace("%%%not-base64%%%"),
        None,
    )
    .expect_err("invalid archive should fail");

    assert!(
        err.to_string()
            .contains("failed decoding staged workspace archive"),
        "unexpected error: {err:#}"
    );
}
