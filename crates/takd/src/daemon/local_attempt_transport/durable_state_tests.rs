use std::path::PathBuf;

use super::durable_state::{AttemptOwner, observe};
use super::workspace::{mark_started, write_completion};
use crate::daemon::attempt_coordinator::AttemptObservation;
use crate::daemon::scheduler::AttemptCompletion;

#[test]
fn live_owner_lock_fences_duplicate_wrappers() {
    let (_temp, root) = attempt_root();
    let owner = AttemptOwner::try_acquire(&root).unwrap().unwrap();

    assert!(AttemptOwner::try_acquire(&root).unwrap().is_none());
    assert_eq!(observe(&root).unwrap(), AttemptObservation::Running);

    drop(owner);
    assert!(AttemptOwner::try_acquire(&root).unwrap().is_some());
}

#[test]
fn an_unlocked_started_attempt_is_missing() {
    let (_temp, root) = attempt_root();
    let owner = AttemptOwner::try_acquire(&root).unwrap().unwrap();
    mark_started(&root).unwrap();
    drop(owner);

    assert_eq!(observe(&root).unwrap(), AttemptObservation::Missing);
    assert_private(&root.join("started"));
}

#[test]
fn terminal_record_wins_over_the_owner_lock() {
    let (_temp, root) = attempt_root();
    let _owner = AttemptOwner::try_acquire(&root).unwrap().unwrap();
    let completion = AttemptCompletion::Succeeded {
        terminal_digest: "digest".into(),
    };
    write_completion(&root, &completion).unwrap();

    assert_eq!(
        observe(&root).unwrap(),
        AttemptObservation::Completed(completion)
    );
    assert_private(&root.join("terminal.json"));
}

fn attempt_root() -> (tempfile::TempDir, PathBuf) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let root = temp.path().join("attempt");
    (temp, root)
}

#[cfg(unix)]
fn assert_private(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[cfg(not(unix))]
fn assert_private(_path: &std::path::Path) {}
