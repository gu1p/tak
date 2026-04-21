use crate::support;

use std::fs;
use std::process::Command as StdCommand;
use std::thread;
use std::time::Duration;

use tak_proto::encode_tor_invite;

#[test]
fn token_show_waits_for_hidden_service_token() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let token_path = state_root.join("agent.token");
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode invite");

    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        fs::write(token_path, format!("{invite}\n")).expect("write invite");
    });

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "2",
        ])
        .output()
        .expect("run takd token show --wait");
    writer.join().expect("writer should exit");

    assert!(
        show.status.success(),
        "takd token show --wait should succeed"
    );
    assert!(
        String::from_utf8_lossy(&show.stdout)
            .trim()
            .starts_with("takd:tor:"),
        "unexpected stdout:\n{}",
        String::from_utf8_lossy(&show.stdout)
    );
}

#[cfg(unix)]
#[test]
fn token_show_wait_reports_unreadable_token_file_instead_of_retrying_as_not_ready() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let token_path = state_root.join("agent.token");
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode invite");
    fs::write(&token_path, format!("{invite}\n")).expect("write invite");
    let mut permissions = fs::metadata(&token_path)
        .expect("token metadata")
        .permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&token_path, permissions).expect("set unreadable permissions");

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--wait",
            "--timeout-secs",
            "1",
        ])
        .output()
        .expect("run takd token show --wait");

    let mut restore = fs::metadata(&token_path)
        .expect("token metadata")
        .permissions();
    restore.set_mode(0o600);
    fs::set_permissions(&token_path, restore).expect("restore permissions");

    assert!(!show.status.success(), "unreadable token should fail");
    let stderr = String::from_utf8_lossy(&show.stderr);
    assert!(
        !stderr.contains("not ready"),
        "stderr should not mask unreadable token as not ready:\n{stderr}"
    );
}
