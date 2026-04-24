use crate::support;

use std::net::TcpListener;

use support::remote_add::{run_add_words_script, spawn_node_info_probe, tor_words};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn remote_add_words_without_values_opens_tui_and_supports_undo() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let words = tor_words()
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let server = spawn_node_info_probe(listener, "builder-undo", V3_BASE_URL.to_string(), "tor");
    let script = format!(
        "word:{},word:{},undo,word:{},paste:{},enter",
        words[0],
        words[1],
        words[1],
        words[2..].join(" ")
    );

    let output = run_add_words_script(
        &config_root,
        &script,
        &[("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))],
    )
    .expect("run scripted words add");

    assert!(
        output.status.success(),
        "tak remote add --words should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Tor invite words"),
        "missing words UI:\n{stdout}"
    );
    assert!(
        stdout.contains("Removed word 02"),
        "missing undo feedback:\n{stdout}"
    );
    assert!(
        stdout.contains("added remote builder-undo"),
        "missing success:\n{stdout}"
    );
    server.join().expect("probe server should exit");
}
