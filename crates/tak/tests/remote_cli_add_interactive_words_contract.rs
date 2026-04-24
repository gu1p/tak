use crate::support;

use std::net::TcpListener;

use support::remote_add::{run_add_script, spawn_node_info_probe, tor_words};
use support::remote_cli::remote_inventory_path;

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn remote_add_no_args_selects_words_confirms_and_persists_remote() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let server = spawn_node_info_probe(
        listener,
        "builder-words-tui",
        V3_BASE_URL.to_string(),
        "tor",
    );

    let output = run_add_script(
        &config_root,
        &format!("enter,paste:{},enter", tor_words()),
        &[("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))],
    )
    .expect("run scripted add");

    assert_success(&output);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Add Remote"),
        "missing method menu:\n{stdout}"
    );
    assert!(stdout.contains("Words"), "missing words method:\n{stdout}");
    assert!(
        stdout.contains("Token or location"),
        "missing location method:\n{stdout}"
    );
    assert!(
        stdout.contains("Tor invite words"),
        "missing words UI:\n{stdout}"
    );
    assert!(
        stdout.contains("01") && stdout.contains("19"),
        "missing numbered cells:\n{stdout}"
    );
    assert!(
        stdout.contains("Confirm Remote"),
        "missing confirmation:\n{stdout}"
    );
    assert!(
        stdout.contains("builder-words-tui"),
        "missing node info:\n{stdout}"
    );
    assert!(
        stdout.contains("added remote builder-words-tui"),
        "missing success:\n{stdout}"
    );
    let inventory =
        std::fs::read_to_string(remote_inventory_path(&config_root)).expect("inventory");
    assert!(inventory.contains("builder-words-tui"));
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
