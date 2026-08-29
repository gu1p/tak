use std::cell::Cell;
use std::fs;
use std::time::Duration;

use anyhow::anyhow;

use super::{quarantine, remove_stale_entry_with_session_fence_with};
use crate::daemon::remote::{
    RemoteNodeContext, SESSION_PATHS_DIR_NAME, SESSION_WORKSPACES_DIR_NAME,
};

#[test]
fn fresh_idle_session_is_not_quarantined() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_root = temp.path().join(SESSION_WORKSPACES_DIR_NAME);
    fs::create_dir(&session_root).expect("create session root");
    let context = RemoteNodeContext::isolated_for_test();
    let quarantine_called = Cell::new(false);

    remove_stale_entry_with_session_fence_with(
        &context,
        &session_root,
        Duration::from_secs(600),
        |_| {
            quarantine_called.set(true);
            unreachable!()
        },
        |_| unreachable!(),
    )
    .expect("skip fresh session");

    assert!(!quarantine_called.get());
    assert!(session_root.exists());
}

#[test]
fn active_execution_prevents_session_quarantine() {
    let temp = tempfile::tempdir().expect("tempdir");
    let context = RemoteNodeContext::isolated_for_test();
    let _cancellation = context
        .register_active_execution("active-submit".into(), "active-run", 1)
        .expect("register active execution");
    for directory in [SESSION_WORKSPACES_DIR_NAME, SESSION_PATHS_DIR_NAME] {
        let session_root = temp.path().join(directory);
        fs::create_dir(&session_root).expect("create session root");
        let quarantine_called = Cell::new(false);

        remove_stale_entry_with_session_fence_with(
            &context,
            &session_root,
            Duration::ZERO,
            |_| {
                quarantine_called.set(true);
                unreachable!()
            },
            |_| unreachable!(),
        )
        .expect("skip active session");

        assert!(!quarantine_called.get());
        assert!(session_root.exists());
    }
}

#[test]
fn delete_failure_leaves_the_tombstone_with_registry_unlocked() {
    let temp = tempfile::tempdir().expect("tempdir");
    let session_root = temp.path().join(SESSION_WORKSPACES_DIR_NAME);
    fs::create_dir(&session_root).expect("create session root");
    let context = RemoteNodeContext::isolated_for_test();
    let tombstone = std::cell::RefCell::new(None);

    let error = remove_stale_entry_with_session_fence_with(
        &context,
        &session_root,
        Duration::ZERO,
        quarantine::quarantine_stale_remote_entry,
        |path| {
            assert!(context.active_execution_registry_is_unlocked_for_test());
            tombstone.replace(Some(path.to_path_buf()));
            Err(anyhow!("delete failed"))
        },
    )
    .expect_err("delete failure");

    assert!(format!("{error:#}").contains("delete failed"));
    assert!(!session_root.exists());
    assert!(
        tombstone
            .borrow()
            .as_ref()
            .is_some_and(|path| path.exists())
    );
}
