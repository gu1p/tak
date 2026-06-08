#![cfg(test)]

use super::*;

use std::fs;

#[test]
fn records_reads_and_commits_removing_backups() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let backup = root.join("takd.bak");
    fs::write(&backup, b"old").unwrap();

    record_pending(root, "v0.1.40", std::slice::from_ref(&backup)).unwrap();
    let pending = read_pending(root).expect("pending recorded");
    assert_eq!(pending.tag, "v0.1.40");
    assert_eq!(pending.boot_attempts, 0);

    commit(root);
    assert!(read_pending(root).is_none(), "state cleared on commit");
    assert!(!backup.exists(), "commit removes backups");
}

#[test]
fn proceeds_until_max_then_rolls_back() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let target = root.join("takd");
    let backup = root.join("takd.bak");
    fs::write(&target, b"new-broken").unwrap();
    fs::write(&backup, b"old-good").unwrap();

    record_pending(root, "v0.1.40", std::slice::from_ref(&backup)).unwrap();
    for _ in 0..MAX_BOOT_ATTEMPTS {
        assert_eq!(reconcile_on_start(root), BootDecision::Proceed);
    }
    assert_eq!(reconcile_on_start(root), BootDecision::RolledBack);
    assert_eq!(
        fs::read(&target).unwrap(),
        b"old-good",
        "backup restored over target"
    );
    assert!(read_pending(root).is_none(), "state cleared after rollback");
}
