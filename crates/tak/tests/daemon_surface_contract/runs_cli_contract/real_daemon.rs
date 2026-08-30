use std::collections::{BTreeMap, HashMap};
use std::fs;

use tak_core::model::WorkspaceSpec;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{run_tak_output, write_tasks};

#[test]
fn runs_client_and_real_daemon_share_the_active_strict_v2_boundary() {
    let root = tempfile::tempdir().expect("temp root");
    write_tasks(root.path(), "raise RuntimeError('TASKS_POISON')\n").expect("write poison tasks");
    let socket = root.path().join("takd.sock");
    let spec = WorkspaceSpec {
        project_id: "empty".into(),
        root: root.path().to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    };
    let _daemon = LocalDaemonGuard::spawn(&socket, &spec);
    let destination = root.path().join("outputs");
    fs::create_dir(&destination).expect("create output destination");
    fs::write(destination.join("keep.txt"), "keep").expect("seed output destination");
    let destination_arg = destination.display().to_string();
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);

    let list = run_tak_output(root.path(), &["runs", "list"], &env).expect("list real runs");
    assert!(
        list.status.success(),
        "{}",
        String::from_utf8_lossy(&list.stderr)
    );
    assert!(list.stdout.is_empty() && list.stderr.is_empty());

    let outputs = run_tak_output(
        root.path(),
        &["runs", "outputs", "run-1", "--to", &destination_arg],
        &env,
    )
    .expect("retrieve missing run");
    assert!(!outputs.status.success());
    let stderr = String::from_utf8_lossy(&outputs.stderr);
    assert!(stderr.contains("run not found"), "{stderr}");
    assert!(!stderr.contains("protocol mismatch") && !stderr.contains("TASKS_POISON"));
    assert_eq!(
        fs::read_to_string(destination.join("keep.txt")).unwrap(),
        "keep"
    );
    assert_eq!(fs::read_dir(destination).unwrap().count(), 1);
}
