use crate::support;

use std::{
    fs,
    path::Path,
    process::{Command as StdCommand, Output},
    thread,
    time::Duration,
};

use tak_proto::encode_tor_invite;

#[test]
fn token_show_waits_for_hidden_service_token() {
    let (_temp, state_root) = state_root();
    let token_path = state_root.join("agent.token");
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode invite");
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        fs::write(token_path, format!("{invite}\n")).expect("write invite");
    });
    let show = token_show_wait(&state_root, "2");
    writer.join().expect("writer should exit");

    assert!(show.status.success());
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.trim().starts_with("takd:tor:"));
}

#[test]
fn token_show_wait_reports_invalid_token_file_instead_of_retrying_as_not_ready() {
    let (_temp, state_root) = state_root();
    fs::write(state_root.join("agent.token"), "not-a-valid-token\n").expect("write invalid token");
    assert_fails_without_not_ready(&token_show_wait(&state_root, "1"));
}

#[cfg(unix)]
#[test]
fn token_show_wait_reports_unreadable_token_file_instead_of_retrying_as_not_ready() {
    use std::os::unix::fs::PermissionsExt;

    let (_temp, state_root) = state_root();
    let token_path = state_root.join("agent.token");
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode invite");
    fs::write(&token_path, format!("{invite}\n")).expect("write invite");
    let mut permissions = fs::metadata(&token_path)
        .expect("token metadata")
        .permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&token_path, permissions).expect("set unreadable permissions");

    if fs::read_to_string(&token_path).is_ok() {
        restore_readable(&token_path);
        eprintln!("skipping chmod unreadable-token contract because mode 000 is still readable");
        return;
    }

    let show = token_show_wait(&state_root, "1");
    restore_readable(&token_path);
    assert_fails_without_not_ready(&show);
}

fn state_root() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    (temp, state_root)
}

fn token_show_wait(state_root: &Path, timeout: &str) -> Output {
    let state_root = state_root.display().to_string();
    let args = [
        "token",
        "show",
        "--state-root",
        &state_root,
        "--wait",
        "--timeout-secs",
        timeout,
    ];
    StdCommand::new(support::takd_bin())
        .args(args)
        .output()
        .expect("run takd token show --wait")
}

#[cfg(unix)]
fn restore_readable(token_path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut restore = fs::metadata(token_path)
        .expect("token metadata")
        .permissions();
    restore.set_mode(0o600);
    fs::set_permissions(token_path, restore).expect("restore permissions");
}

fn assert_fails_without_not_ready(show: &Output) {
    assert!(!show.status.success());
    assert!(!String::from_utf8_lossy(&show.stderr).contains("not ready"));
}
