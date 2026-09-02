use sha2::{Digest, Sha256};
use tak_core::v2::{WorkspaceEntry, WorkspaceManifest};

use super::{CheckoutContext, RunCheckoutStore};

#[test]
fn association_survives_reopen_and_rejects_a_different_replay() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let checkout = temp.path().join("checkout");
    std::fs::create_dir(&checkout).unwrap();
    let socket = temp.path().join("takd.sock");
    let manifest = WorkspaceManifest::new(vec![file("TASKS.py", b"spec")]).unwrap();
    let context = CheckoutContext::new(&checkout, manifest.clone()).unwrap();
    let store = RunCheckoutStore::at(temp.path().join("state"));

    store.record(&socket, "run-1", &context).unwrap();
    store.record(&socket, "run-1", &context).unwrap();

    assert_eq!(store.load(&socket, "run-1").unwrap(), Some(context));
    let other = temp.path().join("other");
    std::fs::create_dir(&other).unwrap();
    let changed = CheckoutContext::new(&other, manifest).unwrap();
    let error = store.record(&socket, "run-1", &changed).unwrap_err();
    assert!(error.to_string().contains("different checkout"));
}

#[cfg(unix)]
#[test]
fn association_files_are_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let checkout = temp.path().join("checkout");
    std::fs::create_dir(&checkout).unwrap();
    let context = CheckoutContext::new(
        &checkout,
        WorkspaceManifest::new(vec![file("TASKS.py", b"spec")]).unwrap(),
    )
    .unwrap();
    let state = temp.path().join("state");
    let store = RunCheckoutStore::at(state.clone());
    store
        .record(&temp.path().join("takd.sock"), "run-1", &context)
        .unwrap();
    let daemon = std::fs::read_dir(&state)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let record = std::fs::read_dir(daemon)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(
        std::fs::metadata(state).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        std::fs::metadata(record).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

fn file(path: &str, contents: &[u8]) -> WorkspaceEntry {
    WorkspaceEntry::file(
        path,
        false,
        contents.len() as u64,
        &format!("{:x}", Sha256::digest(contents)),
    )
    .unwrap()
}
