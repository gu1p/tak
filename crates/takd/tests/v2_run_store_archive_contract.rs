use sha2::{Digest, Sha256};
use tak_core::v2::RunSubmission;
use takd::RunStore;

use crate::support::v2_run::submission;

#[test]
fn uploaded_archive_must_match_the_canonical_workspace_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let store = RunStore::with_db_path(temp.path().join("takd.sqlite")).unwrap();
    let bytes = mismatching_archive();
    let mut request = submission("archive-mismatch", "secret");
    request.run.workspace.archive_sha256 = format!("{:x}", Sha256::digest(&bytes));
    request.run.workspace.archive_size = bytes.len() as u64;
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    let accepted = store.submit(&request, "uid:1").unwrap();

    let error = store
        .upload_workspace(
            &accepted.run_id,
            &request.run.workspace.manifest.fingerprint,
            bytes.len() as u64,
            0,
            &bytes,
        )
        .unwrap_err();

    assert!(error.to_string().contains("manifest"), "{error}");
    assert_eq!(
        store
            .summary(&accepted.run_id)
            .unwrap()
            .unwrap()
            .state
            .as_str(),
        "awaiting_workspace"
    );
}

fn mismatching_archive() -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut builder = tar::Builder::new(&mut bytes);
    builder.mode(tar::HeaderMode::Deterministic);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    header.set_size(4);
    header.set_cksum();
    builder
        .append_data(&mut header, "TASKS.py", &b"evil"[..])
        .unwrap();
    builder.finish().unwrap();
    drop(builder);
    bytes
}
