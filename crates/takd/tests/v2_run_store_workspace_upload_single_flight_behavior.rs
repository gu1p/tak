use std::sync::{Arc, Barrier};

use tak_core::v2::RunSubmission;
use tak_proto::local_daemon::v2::WorkspaceDisposition;
use takd::{RunStore, SubmitRunResult};

use crate::support::v2_run::{ARCHIVE, submission};

#[test]
fn concurrent_runs_share_one_resumable_workspace_upload_across_restart() {
    let temp = tempfile::tempdir().unwrap();
    let db = temp.path().join("takd.sqlite");
    let store = RunStore::with_db_path(db.clone()).unwrap();
    let requests = [
        submission("upload-a", "secret"),
        submission("upload-b", "secret"),
    ];
    let submitted = concurrently_submit(&store, &requests);
    let fingerprint = requests[0].run.workspace.manifest.fingerprint.clone();
    let barrier = Arc::new(Barrier::new(2));
    let uploads = submitted
        .iter()
        .map(|accepted| {
            let store = store.clone();
            let run_id = accepted.run_id.clone();
            let fingerprint = fingerprint.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .upload_workspace(
                        &run_id,
                        &fingerprint,
                        ARCHIVE.len() as u64,
                        0,
                        &ARCHIVE[..16],
                    )
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let progress = uploads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        progress.iter().filter(|item| item.chunk_accepted).count(),
        1
    );
    assert!(progress.iter().all(|item| item.next_offset == 16));
    drop(store);

    let restored = RunStore::with_db_path(db).unwrap();
    for (index, request) in requests.iter().enumerate() {
        let resumed = restored
            .submit(request, if index == 0 { "alice" } else { "bob" })
            .unwrap();
        assert_eq!(
            resumed.workspace,
            WorkspaceDisposition::UploadRequired { next_offset: 16 }
        );
    }
    restored
        .upload_workspace(
            &submitted[1].run_id,
            &fingerprint,
            ARCHIVE.len() as u64,
            16,
            &ARCHIVE[16..],
        )
        .unwrap();
    for accepted in submitted {
        assert_eq!(
            restored.commit(&accepted.run_id).unwrap().state.as_str(),
            "queued"
        );
    }
}

fn concurrently_submit(store: &RunStore, requests: &[RunSubmission; 2]) -> Vec<SubmitRunResult> {
    let barrier = Arc::new(Barrier::new(2));
    requests
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, request)| {
            let store = store.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .submit(&request, if index == 0 { "alice" } else { "bob" })
                    .unwrap()
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect()
}
