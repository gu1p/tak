use std::fs;
use std::os::unix::fs::{MetadataExt, symlink};
use std::path::Path;

use tokio::net::UnixListener;

#[tokio::test]
async fn daemon_start_preserves_regular_file_at_socket_path() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("takd.sock");
    fs::write(&socket, b"user data").unwrap();

    assert!(start_and_stop(temp.path(), &socket).await.is_err());
    assert_eq!(fs::read(socket).unwrap(), b"user data");
}

#[tokio::test]
async fn daemon_start_preserves_symlink_at_socket_path() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("takd.sock");
    let target = temp.path().join("missing-target");
    symlink(&target, &socket).unwrap();

    assert!(start_and_stop(temp.path(), &socket).await.is_err());
    assert_eq!(fs::read_link(socket).unwrap(), target);
}

#[tokio::test]
async fn daemon_start_preserves_live_socket_from_another_state_root() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("takd.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let inode = fs::symlink_metadata(&socket).unwrap().ino();

    assert!(start_and_stop(temp.path(), &socket).await.is_err());
    assert_eq!(fs::symlink_metadata(&socket).unwrap().ino(), inode);
    let _connection = tokio::net::UnixStream::connect(&socket).await.unwrap();
    listener.accept().await.unwrap();
}

#[tokio::test]
async fn daemon_start_recovers_stale_socket() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("takd.sock");
    drop(UnixListener::bind(&socket).unwrap());

    start_and_stop(temp.path(), &socket).await.unwrap();
}

async fn start_and_stop(root: &Path, socket: &Path) -> anyhow::Result<()> {
    let db = root.join("takd.sqlite");
    let manager = takd::new_shared_manager_with_db(db.clone())?;
    let store = takd::RunStore::with_db_path(db)?;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    shutdown_tx.send(()).unwrap();
    takd::run_server_with_local_attempt_executable_and_remote_inventory_until_shutdown(
        socket,
        manager,
        takd::TorBroker::for_direct_dial("127.0.0.1:9"),
        takd::PeerManager::default(),
        store,
        std::env::current_exe()?,
        root.join("remotes.toml"),
        shutdown_rx,
    )
    .await
}
