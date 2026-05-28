use prost::Message;
use sha2::{Digest, Sha256};
use takd::{RemoteRuntimeConfig, handle_remote_v1_request};

#[path = "resumable_workspace_upload_contract/support.rs"]
mod support;

use support::{patch_chunk, post_begin, post_finish, submit_with_upload};

#[test]
fn workspace_upload_begin_append_and_finish_resume_by_offset() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store =
        takd::SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let runtime = RemoteRuntimeConfig::for_tests().with_temp_dir(temp.path());
    let context = crate::support::remote_output::test_context_with_runtime(runtime)
        .with_state_root(temp.path());
    let archive = b"workspace-zip-bytes";
    let digest = format!("{:x}", Sha256::digest(archive));

    let begin = post_begin(&context, &store, &digest, archive.len() as u64);
    patch_chunk(&context, &store, &begin.upload_id, 0, &archive[..9]);
    let resumed = post_begin(&context, &store, &digest, archive.len() as u64);
    assert_eq!(resumed.offset, 9);

    patch_chunk(&context, &store, &begin.upload_id, 9, &archive[9..]);
    let finished = post_finish(&context, &store, &begin.upload_id);
    let repeated_finish = post_finish(&context, &store, &begin.upload_id);

    assert!(finished.complete);
    assert_eq!(finished.size_bytes, archive.len() as u64);
    assert!(repeated_finish.complete);
    assert_eq!(repeated_finish.size_bytes, archive.len() as u64);
}

#[test]
fn workspace_upload_finish_completes_zero_byte_upload() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store =
        takd::SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let runtime = RemoteRuntimeConfig::for_tests().with_temp_dir(temp.path());
    let context = crate::support::remote_output::test_context_with_runtime(runtime)
        .with_state_root(temp.path());
    let digest = format!("{:x}", Sha256::digest([]));

    let begin = post_begin(&context, &store, &digest, 0);
    let finished = post_finish(&context, &store, &begin.upload_id);

    assert!(finished.complete);
    assert_eq!(finished.size_bytes, 0);
}

#[test]
fn submit_rejects_workspace_upload_id_path_escape() {
    let temp = tempfile::tempdir().expect("tempdir");
    let store =
        takd::SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let runtime = RemoteRuntimeConfig::for_tests().with_temp_dir(temp.path());
    let context = crate::support::remote_output::test_context_with_runtime(runtime)
        .with_state_root(temp.path());
    let archive = crate::support::remote_output::empty_workspace_zip();
    let digest = format!("{:x}", Sha256::digest(&archive));
    let exec_root = temp.path().join("takd-remote-exec");
    std::fs::create_dir_all(&exec_root).expect("exec root");
    std::fs::write(exec_root.join("escape.zip"), &archive).expect("escaped upload");

    let submit = submit_with_upload("../escape", &digest, archive.len() as u64);
    let response = handle_remote_v1_request(
        &context,
        &store,
        "POST",
        "/v1/tasks/submit",
        Some(&submit.encode_to_vec()),
    )
    .expect("submit response");

    assert_eq!(response.status_code, 400);
}
