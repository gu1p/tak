use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn docker_run_returns_the_daemon_terminal_exit_code() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::TerminalOutputFlow("failed", false));
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &[
            "--local",
            "docker",
            "run",
            "alpine:3.20",
            "sh",
            "-c",
            "exit 7",
        ],
        &env,
    )
    .unwrap();
    daemon.finish_expecting(6);

    assert_eq!(
        output.status.code(),
        Some(7),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
