use std::collections::BTreeSet;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::time::Duration;

use super::cleanup_stale_remote_entries_with;

#[test]
fn cleanup_janitor_skips_permission_denied_entry_and_continues_other_roots() {
    let temp = tempfile::tempdir().expect("tempdir");
    let denied_root = temp.path().join("denied-job");
    let stale_root = temp.path().join("stale-job");
    std::fs::create_dir_all(&denied_root).expect("create denied root");
    std::fs::create_dir_all(&stale_root).expect("create stale root");

    let mut attempted_denied = false;
    cleanup_stale_remote_entries_with(temp.path(), &BTreeSet::new(), Duration::ZERO, |path| {
        remove_or_deny(path, &denied_root, &mut attempted_denied)
    })
    .expect("cleanup skips permission denied entries");

    assert!(attempted_denied);
    assert!(denied_root.exists());
    assert!(!stale_root.exists());
}

fn remove_or_deny(
    path: &Path,
    denied_root: &Path,
    attempted_denied: &mut bool,
) -> anyhow::Result<()> {
    if path == denied_root {
        *attempted_denied = true;
        return Err(Error::new(ErrorKind::PermissionDenied, "owned by another user").into());
    }

    std::fs::remove_dir_all(path)?;
    Ok(())
}
