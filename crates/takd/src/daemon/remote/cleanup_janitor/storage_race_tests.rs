use super::storage;

#[test]
fn removing_an_already_vanished_stale_entry_is_benign() {
    let temp = tempfile::tempdir().expect("tempdir");
    let vanished = temp.path().join("vanished-tombstone");

    storage::remove_stale_remote_entry(&vanished).expect("ignore vanished tombstone");
}
