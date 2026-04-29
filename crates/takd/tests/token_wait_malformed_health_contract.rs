use std::fs;

use tak_proto::encode_tor_invite;

use crate::token_wait_transport_contract::{state_root, token_show_wait};

#[test]
fn token_show_wait_reports_unready_transport_when_health_file_is_empty() {
    let (_temp, state_root) = state_root();
    let base_url = "http://builder-a.onion";
    let invite = encode_tor_invite(base_url).expect("encode invite");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");
    fs::write(state_root.join("transport-health.toml"), "").expect("write empty health");

    let show = token_show_wait(&state_root, "0");

    assert!(!show.status.success());
    let stderr = String::from_utf8_lossy(&show.stderr);
    assert!(stderr.contains("tor transport is recovering"));
    assert!(stderr.contains("transport health file is unreadable"));
    assert!(!stderr.contains("Error: decode"));
}
