use std::collections::{BTreeMap, HashMap};
use std::fs;

use crate::support::local_daemon::LocalDaemonGuard;
use tak_core::model::WorkspaceSpec;

#[test]
fn checkout_local_absolute_socket_path_may_exceed_sun_len() {
    let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("checkout root");
    let test_tmp = checkout.join(".tmp");
    fs::create_dir_all(&test_tmp).expect("checkout test temp root");
    let temp = tempfile::tempdir_in(test_tmp).expect("checkout-local tempdir");
    let mut root = temp.path().join("socket-root");
    while root.join("takd.sock").to_string_lossy().len() <= 104 {
        root.push("x");
    }
    fs::create_dir_all(&root).expect("long socket parent");
    let socket = root.join("takd.sock");
    assert!(
        socket.to_string_lossy().len() > 104,
        "test path must be long"
    );
    let spec = WorkspaceSpec {
        project_id: "long-socket".into(),
        root: root.clone(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    };

    let daemon = LocalDaemonGuard::spawn(&socket, &spec);
    assert!(daemon.effective_socket_path().is_relative());
    assert!(
        daemon.effective_socket_path().starts_with("../.."),
        "package tests must reach the repo-root socket through a common ancestor"
    );
    assert!(
        socket.exists(),
        "relative bind must create the absolute socket"
    );
}
