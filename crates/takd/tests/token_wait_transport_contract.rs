use crate::support;

use std::{
    fs,
    path::Path,
    process::{Command as StdCommand, Output},
    thread,
    time::{Duration, Instant},
};

use tak_proto::encode_tor_invite;
use takd::agent::{TransportHealth, TransportState, write_transport_health};

#[test]
fn token_show_waits_for_hidden_service_token_and_ready_transport() {
    let (_temp, state_root) = state_root();
    let token_path = state_root.join("agent.token");
    let state_for_writer = state_root.clone();
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode invite");
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(250));
        fs::write(token_path, format!("{invite}\n")).expect("write invite");
        write_transport_health(
            &state_for_writer,
            &TransportHealth::ready(Some("http://builder-a.onion".to_string())),
        )
        .expect("write ready transport health");
    });
    let started_at = Instant::now();
    let show = token_show_wait(&state_root, "2");
    writer.join().expect("writer should exit");

    assert!(show.status.success());
    assert!(started_at.elapsed() >= Duration::from_millis(200));
    assert!(
        String::from_utf8_lossy(&show.stdout)
            .trim()
            .starts_with("takd:tor:")
    );
}

#[test]
fn token_show_wait_keeps_waiting_while_tor_transport_is_recovering() {
    let (_temp, state_root) = state_root();
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode invite");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");
    write_transport_health(
        &state_root,
        &TransportHealth::new(
            TransportState::Recovering,
            Some("http://builder-a.onion".to_string()),
            Some("self-probe failed: rendezvous circuit timed out".to_string()),
        ),
    )
    .expect("write recovering transport health");

    let show = token_show_wait(&state_root, "0");

    assert!(!show.status.success());
    let stderr = String::from_utf8_lossy(&show.stderr);
    assert!(stderr.contains("tor transport is recovering"));
    assert!(stderr.contains("self-probe failed: rendezvous circuit timed out"));
}

pub(crate) fn state_root() -> (tempfile::TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    (temp, state_root)
}

pub(crate) fn token_show_wait(state_root: &Path, timeout: &str) -> Output {
    let state_root = state_root.display().to_string();
    StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root,
            "--wait",
            "--timeout-secs",
            timeout,
        ])
        .output()
        .expect("run takd token show --wait")
}
