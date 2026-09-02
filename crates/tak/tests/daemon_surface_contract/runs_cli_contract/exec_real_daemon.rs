use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

use tak_core::model::WorkspaceSpec;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::run_tak_output;

#[test]
fn exec_local_work_runs_inside_the_daemon() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("work")).unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
        ("EXEC_TOKEN".into(), "passed".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &[
            "exec", "--cwd", "work", "--env", "INLINE=override", "--pass-env",
            "EXEC_TOKEN", "--", "/bin/sh", "-c",
            "test \"$INLINE:$EXEC_TOKEN\" = override:passed && printf 'daemon-exec-marker\\n' && touch undeclared",
        ],
        &env,
    )
    .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{stdout}\n{stderr}");
    assert!(stdout.contains("run_id="), "{stdout}");
    assert!(stdout.contains("daemon-exec-marker"), "{stdout}");
    assert!(!workspace.join("work/undeclared").exists());
}

#[test]
fn exec_returns_the_wrapped_process_exit_code_from_takd() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
        ("EXIT_SECRET".into(), "must-not-render".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &[
            "exec",
            "--pass-env",
            "EXIT_SECRET",
            "--",
            "/bin/sh",
            "-c",
            "exit 7",
        ],
        &env,
    )
    .unwrap();

    assert_eq!(output.status.code(), Some(7));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("must-not-render"));
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "exec-v2".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}
