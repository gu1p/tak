use super::submit_payload_test_support::{
    direct_target, missing_archive_workspace, task_with_steps_and_needs,
};
use super::*;

#[test]
fn build_remote_submit_payload_rejects_missing_workspace_archive() {
    let target = direct_target(None);
    let task = task_with_steps_and_needs();
    let remote_workspace = missing_archive_workspace();
    let err = build_remote_submit_payload(RemoteSubmitPayloadInput {
        target: &target,
        task_run_id: "task-run-1",
        attempt: 1,
        task: &task,
        remote_workspace: Some(&remote_workspace),
        session: None,
        execution_label: None,
        fused_members: None,
        fused_member_execution_labels: None,
        workspace_upload: None,
    })
    .expect_err("missing archive should fail");

    assert!(
        err.to_string()
            .contains("failed reading staged workspace archive"),
        "unexpected error: {err:#}"
    );
}
