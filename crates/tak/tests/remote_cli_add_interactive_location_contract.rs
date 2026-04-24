use crate::support;

use std::net::TcpListener;

use support::remote_add::{run_add_script, spawn_node_info_probe};
use support::remote_cli::{remote_inventory_path, remote_token};

#[test]
fn remote_add_token_or_location_accepts_token_and_confirms_before_save() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let token = remote_token("builder-token-tui", &base_url, "direct");
    let server = spawn_node_info_probe(listener, "builder-token-tui", base_url.clone(), "direct");

    let output = run_add_script(
        &config_root,
        &format!("down,enter,paste:{token},enter,enter"),
        &[],
    )
    .expect("run scripted add");

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Token or location"),
        "missing method:\n{stdout}"
    );
    assert!(
        stdout.contains("Confirm Remote"),
        "missing confirmation:\n{stdout}"
    );
    assert!(
        stdout.contains("builder-token-tui"),
        "missing node info:\n{stdout}"
    );
    assert!(
        stdout.contains("added remote builder-token-tui"),
        "missing success:\n{stdout}"
    );
    let inventory =
        std::fs::read_to_string(remote_inventory_path(&config_root)).expect("inventory");
    assert!(inventory.contains("builder-token-tui"));
    server.join().expect("probe server should exit");
}

#[test]
fn remote_add_cancel_from_confirmation_leaves_inventory_empty() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let addr = listener.local_addr().expect("listener addr");
    let base_url = format!("http://{addr}");
    let token = remote_token("builder-cancel-tui", &base_url, "direct");
    let server = spawn_node_info_probe(listener, "builder-cancel-tui", base_url, "direct");

    let output = run_add_script(
        &config_root,
        &format!("down,enter,paste:{token},enter,esc"),
        &[],
    )
    .expect("run scripted add");

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Confirm Remote"),
        "missing confirmation:\n{stdout}"
    );
    assert!(
        stdout.contains("remote add cancelled"),
        "missing cancel:\n{stdout}"
    );
    assert!(
        !remote_inventory_path(&config_root).exists(),
        "inventory should not be written"
    );
    server.join().expect("probe server should exit");
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "tak remote add should succeed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
