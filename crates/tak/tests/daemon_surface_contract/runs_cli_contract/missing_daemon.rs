use std::collections::BTreeMap;
use std::fs;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn every_runs_command_reports_the_missing_daemon_before_loading_client_state() {
    let root = tempfile::tempdir().expect("temp root");
    write_tasks(
        root.path(),
        "raise RuntimeError('RUNS_MUST_NOT_LOAD_TASKS')\n",
    )
    .expect("write poison tasks");
    let destination = root.path().join("retrieved");
    fs::create_dir(&destination).expect("create destination");
    fs::write(destination.join("keep.txt"), "keep").expect("seed destination");
    let state_home = root.path().join("state");
    fs::create_dir_all(state_home.join("tak")).expect("create state home");
    fs::write(state_home.join("tak/tasks.sqlite"), "CLIENT_HISTORY_POISON")
        .expect("poison history");
    let socket = root.path().join("not-running.sock");
    let destination_arg = destination.display().to_string();
    let cases = [
        vec!["runs", "list"],
        vec!["runs", "show", "run-1"],
        vec!["runs", "attach", "run-1"],
        vec!["runs", "cancel", "run-1"],
        vec!["runs", "outputs", "run-1", "--to", &destination_arg],
    ];
    let env = BTreeMap::from([
        ("TAKD_SOCKET".to_string(), socket.display().to_string()),
        (
            "XDG_STATE_HOME".to_string(),
            state_home.display().to_string(),
        ),
    ]);

    for args in cases {
        let output = run_tak_output(root.path(), &args, &env).expect("run recovery command");
        assert!(!output.status.success(), "{args:?} should fail");
        assert!(output.stdout.is_empty(), "{args:?} wrote stdout");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(&socket.display().to_string()), "{stderr}");
        assert!(stderr.contains("takd serve"), "{stderr}");
        assert!(stderr.contains("no client execution fallback"), "{stderr}");
        assert!(!stderr.contains("RUNS_MUST_NOT_LOAD_TASKS"), "{stderr}");
        assert!(!stderr.contains("CLIENT_HISTORY_POISON"), "{stderr}");
    }

    assert_eq!(
        fs::read_to_string(destination.join("keep.txt")).unwrap(),
        "keep"
    );
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 1);
    assert_eq!(
        fs::read(state_home.join("tak/tasks.sqlite")).unwrap(),
        b"CLIENT_HISTORY_POISON"
    );
}
