use std::collections::{BTreeMap, HashMap};

use tak_core::model::WorkspaceSpec;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::run_tak_output;

#[test]
fn client_connects_to_checkout_local_absolute_socket_beyond_darwin_limit() {
    let checkout = std::env::current_dir().expect("current directory");
    let temp = tempfile::tempdir_in(&checkout).expect("checkout-local tempdir");
    let mut socket_root = temp.path().join("x");
    while socket_root
        .join("takd.sock")
        .as_os_str()
        .as_encoded_bytes()
        .len()
        <= 103
    {
        socket_root.push("x");
    }
    let workspace = socket_root.join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let socket = socket_root.join("takd.sock");
    assert!(
        socket.as_os_str().as_encoded_bytes().len() > 103,
        "test socket must exceed Darwin sun_path"
    );
    assert!(
        socket
            .strip_prefix(&checkout)
            .expect("checkout-local socket")
            .as_os_str()
            .as_encoded_bytes()
            .len()
            <= 103,
        "daemon must have a short checkout-relative bind path"
    );
    let spec = WorkspaceSpec {
        project_id: "long-client-socket".into(),
        root: workspace.clone(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    };
    let _daemon = LocalDaemonGuard::spawn(&socket, &spec);
    let environment = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);

    let output = run_tak_output(&workspace, &["runs", "list"], &environment)
        .expect("run list through long absolute socket");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
