use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::RunStore;

use crate::support::v2_run::{ARCHIVE, submission};

#[test]
fn a_corrupt_resumable_prefix_resets_after_digest_verification_fails() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let request = submission("corrupt-resume", "secret");
    let accepted = store.submit(&request, "uid:1").unwrap();
    let fingerprint = &request.run.workspace.manifest.fingerprint;
    store
        .upload_workspace(
            &accepted.run_id,
            fingerprint,
            ARCHIVE.len() as u64,
            0,
            &ARCHIVE[..16],
        )
        .unwrap();
    std::fs::write(
        db.with_extension("v2-blobs")
            .join("uploads")
            .join(format!("{}.part", accepted.run_id)),
        [b'x'; 16],
    )
    .unwrap();
    assert!(
        store
            .upload_workspace(
                &accepted.run_id,
                fingerprint,
                ARCHIVE.len() as u64,
                16,
                &ARCHIVE[16..],
            )
            .is_err()
    );

    let resumed = store.submit(&request, "uid:1").unwrap();
    assert_eq!(
        resumed.workspace,
        WorkspaceDisposition::UploadRequired { next_offset: 0 }
    );
    store
        .upload_workspace(
            &accepted.run_id,
            fingerprint,
            ARCHIVE.len() as u64,
            0,
            ARCHIVE.as_slice(),
        )
        .unwrap();
    assert_eq!(
        store.commit(&accepted.run_id).unwrap().state.as_str(),
        "queued"
    );
}
