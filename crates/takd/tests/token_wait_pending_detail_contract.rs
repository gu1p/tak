use std::fs;

use tak_proto::encode_tor_invite;
use takd::agent::{TransportHealth, TransportState, write_transport_health};

use crate::token_wait_transport_contract::{state_root, token_show_wait};

#[test]
fn token_show_wait_reports_pending_tor_transport_detail() {
    let (_temp, state_root) = state_root();
    let base_url = "http://builder-a.onion";
    let invite = encode_tor_invite(base_url).expect("encode invite");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");
    write_transport_health(
        &state_root,
        &TransportHealth::new(
            TransportState::Pending,
            Some(base_url.to_string()),
            Some("self-probe connect attempt 3 failed: rendezvous circuit timed out".to_string()),
        ),
    )
    .expect("write pending transport health");

    let show = token_show_wait(&state_root, "0");

    assert!(!show.status.success());
    let stderr = String::from_utf8_lossy(&show.stderr);
    assert!(stderr.contains("tor transport is pending"));
    assert!(stderr.contains("self-probe connect attempt 3 failed"));
    assert!(stderr.contains("rendezvous circuit timed out"));
}
