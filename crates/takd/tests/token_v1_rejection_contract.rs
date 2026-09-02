use std::fs;

use crate::token_wait_transport_contract::{state_root, token_show_wait};

#[test]
fn token_show_rejects_stale_v1_state_with_coordinated_upgrade_guidance() {
    let (_temp, state_root) = state_root();
    fs::write(state_root.join("agent.token"), "takd:v1:not-base64\n").unwrap();

    let show = token_show_wait(&state_root, "0");

    assert!(!show.status.success());
    let stderr = String::from_utf8_lossy(&show.stderr);
    assert!(
        stderr.contains("upgrade tak, takd, and workers together"),
        "{stderr}"
    );
    assert!(!stderr.contains("decode remote token base64"), "{stderr}");
}
