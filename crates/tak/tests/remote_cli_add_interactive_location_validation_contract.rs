use crate::support;

use std::net::TcpListener;

use support::remote_add::{run_add_script, spawn_node_info_probe};
use support::remote_cli::{remote_inventory_path, remote_token};

#[test]
fn remote_add_token_or_location_invalid_input_stays_open_and_can_be_corrected() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let token = remote_token("builder-location-retry-tui", &base_url, "direct");
    let server = spawn_node_info_probe(
        listener,
        "builder-location-retry-tui",
        base_url.clone(),
        "direct",
    );

    let output = run_add_script(
        &config_root,
        &format!("down,enter,paste:http://127.0.0.1:3000,enter,ctrl_u,paste:{token},enter,enter"),
        &[],
    )
    .expect("run scripted add");

    assert!(
        output.status.success(),
        "tak remote add should recover from invalid location\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("paste a takd token, takd tor invite, or Tor .onion location"),
        "missing inline validation:\n{stdout}"
    );
    assert!(
        stdout.contains("added remote builder-location-retry-tui"),
        "missing success after correction:\n{stdout}"
    );
    let inventory =
        std::fs::read_to_string(remote_inventory_path(&config_root)).expect("inventory");
    assert!(inventory.contains("builder-location-retry-tui"));
    server.join().expect("probe server should exit");
}
