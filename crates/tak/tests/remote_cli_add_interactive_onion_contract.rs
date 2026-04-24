use crate::support;

use std::net::TcpListener;

use support::remote_add::{run_add_script, spawn_node_info_probe};
use support::remote_cli::remote_inventory_path;

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn remote_add_token_or_location_accepts_tor_onion_url() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let server = spawn_node_info_probe(listener, "builder-onion-url", V3_BASE_URL.into(), "tor");

    let output = run_add_script(
        &config_root,
        &format!("down,enter,paste:{V3_BASE_URL},enter,enter"),
        &[("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))],
    )
    .expect("run scripted add");

    assert!(output.status.success(), "tak remote add should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("added remote builder-onion-url"),
        "missing success:\n{stdout}"
    );
    let inventory =
        std::fs::read_to_string(remote_inventory_path(&config_root)).expect("inventory");
    assert!(inventory.contains("builder-onion-url"));
    server.join().expect("probe server should exit");
}
