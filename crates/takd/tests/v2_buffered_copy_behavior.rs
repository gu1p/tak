use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "../src/daemon/workspace_layer/buffered_copy.rs"]
mod buffered_copy;

#[test]
fn failed_create_never_removes_an_existing_destination() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("tak-buffered-copy-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("source");
    let destination = root.join("destination");
    fs::write(&source, b"new").unwrap();
    fs::write(&destination, b"existing").unwrap();

    let error = buffered_copy::copy(&source, &destination).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read(destination).unwrap(), b"existing");
}
