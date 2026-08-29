use std::fs;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, SystemTime};

use super::{quarantine, remove_stale_entry_with_session_fence_with, storage};
use crate::daemon::remote::{RemoteNodeContext, SESSION_WORKSPACES_DIR_NAME};

#[test]
fn quarantine_fences_registration_but_recursive_removal_does_not() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_root = temp.path().join(SESSION_WORKSPACES_DIR_NAME);
    let old_sentinel = session_root.join("old/session-state");
    fs::create_dir_all(old_sentinel.parent().expect("old session parent"))
        .expect("create old session");
    fs::write(&old_sentinel, b"old").expect("write old session");
    backdate(&session_root);
    let context = RemoteNodeContext::isolated_for_test();
    let recreated = session_root.join("new/session-state");
    let (start_tx, start_rx) = mpsc::channel();
    let (attempting_tx, attempting_rx) = mpsc::channel();
    let (registered_tx, registered_rx) = mpsc::channel();
    let registration_context = context.clone();
    let registration_path = recreated.clone();
    let registration = thread::spawn(move || {
        start_rx
            .recv_timeout(Duration::from_secs(45))
            .expect("start registration");
        attempting_tx.send(()).expect("signal registration attempt");
        let _cancellation = registration_context
            .register_active_execution("new-submit".into(), "new-run", 1)
            .expect("register new execution");
        fs::create_dir_all(registration_path.parent().expect("new session parent"))
            .expect("recreate session root");
        fs::write(registration_path, b"new").expect("write new session");
        registered_tx.send(()).expect("signal registration");
    });

    let cleanup = remove_stale_entry_with_session_fence_with(
        &context,
        &session_root,
        Duration::ZERO,
        |path| {
            assert!(!context.active_execution_registry_is_unlocked_for_test());
            start_tx.send(()).expect("start registration race");
            attempting_rx
                .recv_timeout(Duration::from_secs(45))
                .expect("registration attempting");
            assert!(!context.active_execution_registry_is_unlocked_for_test());
            assert!(matches!(registered_rx.try_recv(), Err(TryRecvError::Empty)));
            quarantine::quarantine_stale_remote_entry(path)
        },
        |tombstone| {
            registered_rx
                .recv_timeout(Duration::from_secs(45))
                .expect("registration did not resume before recursive deletion");
            assert!(context.active_execution_registry_is_unlocked_for_test());
            storage::remove_stale_remote_entry(tombstone)
        },
    );
    drop(start_tx);
    cleanup.expect("quarantine stale session");
    registration.join().expect("registration thread");

    assert!(!old_sentinel.exists());
    assert_eq!(fs::read(recreated).expect("read new session"), b"new");
}

fn backdate(path: &std::path::Path) {
    fs::File::open(path)
        .expect("open path")
        .set_modified(SystemTime::UNIX_EPOCH)
        .expect("backdate path");
}
