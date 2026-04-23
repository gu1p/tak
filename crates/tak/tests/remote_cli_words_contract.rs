use crate::support;

use std::io::Write;
use std::net::TcpListener;
use std::process::Command as StdCommand;
use std::thread;

use prost::Message;
use support::remote_cli::{node_info, read_request, remote_inventory_path};
use tak_proto::{encode_tor_invite, encode_tor_invite_words};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn remote_add_accepts_tor_invite_words_and_persists_remote() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind node info server");
    let port = listener.local_addr().expect("listener addr").port();
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode tor invite");
    let words = encode_tor_invite_words(&invite).expect("encode tor invite words");

    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        let request = read_request(&mut stream);
        assert!(request.starts_with("GET /v1/node/info HTTP/1.1\r\n"));
        assert!(!request.contains("Authorization:"));
        let body = node_info("builder-words", V3_BASE_URL, "tor").encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response");
        stream.write_all(&body).expect("write protobuf body");
    });

    let mut command = StdCommand::new(support::tak_bin());
    command.args(["remote", "add", "--words"]);
    command.args(words.split_whitespace());
    let add = command
        .env("XDG_CONFIG_HOME", &config_root)
        .env("TAK_TEST_TOR_ONION_DIAL_ADDR", format!("127.0.0.1:{port}"))
        .output()
        .expect("run tak remote add --words");

    assert!(
        add.status.success(),
        "tak remote add --words should succeed"
    );
    let inventory = std::fs::read_to_string(config_root.join("tak/remotes.toml"))
        .expect("read persisted inventory");
    assert!(inventory.contains("builder-words"));
    assert!(inventory.contains("bearer_token = \"\""));
    server.join().expect("probe server should exit");
}

#[test]
fn remote_add_rejects_tor_invite_words_with_bad_checksum_before_probe() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode tor invite");
    let words = encode_tor_invite_words(&invite).expect("encode tor invite words");
    let mut phrase = words.split_whitespace().collect::<Vec<_>>();
    let last = phrase.len() - 1;
    phrase[last] = if phrase[last] == "a" { "aa" } else { "a" };

    let mut command = StdCommand::new(support::tak_bin());
    command.args(["remote", "add", "--words"]);
    command.args(phrase);
    let add = command
        .env("XDG_CONFIG_HOME", &config_root)
        .output()
        .expect("run tak remote add --words");

    assert!(!add.status.success(), "bad checksum should fail");
    assert!(String::from_utf8_lossy(&add.stderr).contains("checksum"));
    assert!(!remote_inventory_path(&config_root).exists());
}
