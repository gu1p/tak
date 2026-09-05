use super::socket_binding::bind;

// Tak's attempt-specific TMPDIR can exceed Unix socket path limits.
// Use unique directories under /tmp so these tests also exercise binding in CI.

#[tokio::test]
async fn binding_preserves_existing_files_and_symlinks() {
    let temp = tempfile::tempdir_in("/tmp").unwrap();
    let path = temp.path().join("takd.sock");
    std::fs::write(&path, b"user data").unwrap();
    assert!(bind(&path).await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"user data");
    std::fs::remove_file(&path).unwrap();
    let target = temp.path().join("missing");
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(bind(&path).await.is_err());
    assert_eq!(std::fs::read_link(path).unwrap(), target);
}

#[tokio::test]
async fn binding_preserves_live_listeners_and_recovers_stale_sockets() {
    use std::os::unix::fs::MetadataExt;
    let temp = tempfile::tempdir_in("/tmp").unwrap();
    let path = temp.path().join("takd.sock");
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let inode = std::fs::symlink_metadata(&path).unwrap().ino();
    assert!(bind(&path).await.is_err());
    assert_eq!(std::fs::symlink_metadata(&path).unwrap().ino(), inode);
    drop(listener);
    assert!(bind(&path).await.is_ok());
}

#[tokio::test]
async fn socket_ownership_survives_unlink_until_listener_is_released() {
    let temp = tempfile::tempdir_in("/tmp").unwrap();
    let path = temp.path().join("takd.sock");
    let first = bind(&path).await.unwrap();
    std::fs::remove_file(&path).unwrap();

    assert!(bind(&path).await.is_err());
    drop(first);
    assert!(bind(&path).await.is_ok());
}

#[tokio::test]
async fn socket_lock_does_not_follow_symlinks() {
    let temp = tempfile::tempdir_in("/tmp").unwrap();
    let path = temp.path().join("takd.sock");
    let target = temp.path().join("user-file");
    std::fs::write(&target, b"preserve").unwrap();
    std::os::unix::fs::symlink(&target, path.with_added_extension("lock")).unwrap();

    assert!(bind(&path).await.is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"preserve");
    assert!(!path.exists());
}
