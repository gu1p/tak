use super::submit_payload_test_support::{direct_target, task_with_steps_and_needs, workspace};
use super::*;

#[test]
fn build_remote_submit_payload_rejects_invalid_workspace_archive() {
    let target = direct_target(None);
    let task = task_with_steps_and_needs();
    let remote_workspace = workspace("%%%not-base64%%%");
    let err = build_remote_submit_payload(RemoteSubmitPayloadInput {
        target: &target,
        task_run_id: "task-run-1",
        attempt: 1,
        task: &task,
        remote_workspace: &remote_workspace,
        session: None,
        execution_label: None,
        fused_members: None,
        fused_member_execution_labels: None,
    })
    .expect_err("invalid archive should fail");

    assert!(
        err.to_string()
            .contains("failed decoding staged workspace archive"),
        "unexpected error: {err:#}"
    );
}
