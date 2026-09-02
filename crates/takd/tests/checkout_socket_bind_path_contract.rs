#![cfg(unix)]

use std::os::unix::ffi::OsStrExt;
use std::path::Path;

#[test]
fn long_checkout_socket_uses_a_short_relative_bind_path() {
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let socket = temp.path().join("docker.sock");
    assert!(socket.as_os_str().as_bytes().len() > 103);

    let bind_path = crate::support::socket_path::bind_path(&socket);

    assert!(bind_path.is_relative(), "{}", bind_path.display());
    assert!(bind_path.as_os_str().as_bytes().len() <= 103);
    assert_eq!(
        std::env::current_dir().unwrap().join(bind_path),
        socket,
        "relative bind path must name the requested socket"
    );
}

#[test]
fn short_socket_path_is_preserved() {
    let socket = Path::new(".tmp/docker.sock");
    assert_eq!(crate::support::socket_path::bind_path(socket), socket);
}

#[test]
fn daemon_commands_use_one_fixed_root_and_short_relative_state_paths() {
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let config = temp.path().join("config");
    let state = temp.path().join("state");
    let paths = crate::support::daemon_command_paths::DaemonCommandPaths::new(&config, &state);

    let command = paths.rooted_command(Path::new("/bin/true"), "serve");

    assert_eq!(command.get_current_dir(), Some(temp.path()));
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        args,
        ["serve", "--config-root", "config", "--state-root", "state"]
    );
    assert_eq!(paths.runtime_root(), Path::new("runtime"));
    assert_eq!(paths.remote_exec_root(), Path::new("remote-exec"));
}
