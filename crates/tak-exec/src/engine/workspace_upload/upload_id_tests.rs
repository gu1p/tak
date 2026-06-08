use super::workspace_upload_id;

#[test]
fn workspace_upload_id_combines_task_run_and_archive_hash() {
    assert_eq!(
        workspace_upload_id("task-run-1", "abcdef123456"),
        "task-run-1-abcdef123456"
    );
}

#[test]
fn workspace_upload_id_preserves_allowed_ascii_characters() {
    assert_eq!(
        workspace_upload_id("Run_1.alpha", "sha-256_HASH.1"),
        "Run_1.alpha-sha-256_HASH.1"
    );
}

#[test]
fn workspace_upload_id_replaces_disallowed_and_non_ascii_characters() {
    assert_eq!(
        workspace_upload_id("task/run:1 with café", "sha/256:abc"),
        "task_run_1_with_caf_-sha_256_abc"
    );
}
