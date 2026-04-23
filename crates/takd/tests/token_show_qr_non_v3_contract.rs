use crate::support;

use std::fs;
use std::process::Command as StdCommand;

use tak_proto::encode_tor_invite;

#[test]
fn token_show_qr_keeps_raw_invite_view_for_non_v3_onion_hosts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let invite = encode_tor_invite("http://builder-qr.onion").expect("encode invite");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--qr",
        ])
        .output()
        .expect("run takd token show --qr");

    assert!(show.status.success(), "takd token show --qr should succeed");
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("Scan this QR code"));
    assert!(!stdout.contains("tak remote add --words"));
    assert!(!stdout.contains("┌ Words "));
}
