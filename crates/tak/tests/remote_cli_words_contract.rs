#![cfg(unix)]

use std::process::Command;

use tak_proto::{encode_tor_invite, encode_tor_invite_words};

use crate::support;
use support::remote_daemon_v2::{FakeRemoteDaemon, remote};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn remote_add_accepts_tor_words_via_daemon_v2() {
    let root = tempfile::tempdir().expect("temp root");
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode invite");
    let words = encode_tor_invite_words(&invite).expect("encode words");
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![serde_json::json!({
            "type": "RemoteAdded", "remote": remote("builder-words")
        })],
    );
    let mut command = Command::new(support::tak_bin());
    command.args(["remote", "add", "--words"]);
    command.args(words.split_whitespace());
    let output = command
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("remote add words");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("added remote builder-words"));
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "AddRemote");
    assert_eq!(requests[0]["operation"]["invite"], invite);
}

#[test]
fn remote_add_rejects_bad_word_checksum_before_contacting_daemon() {
    let root = tempfile::tempdir().expect("temp root");
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode invite");
    let words = encode_tor_invite_words(&invite).expect("encode words");
    let mut phrase = words.split_whitespace().collect::<Vec<_>>();
    let last = phrase.len() - 1;
    phrase[last] = if phrase[last] == "a" { "aa" } else { "a" };
    let mut command = Command::new(support::tak_bin());
    command.args(["remote", "add", "--words"]);
    command.args(phrase);
    let output = command
        .env("TAKD_SOCKET", root.path().join("missing.sock"))
        .output()
        .expect("remote add invalid words");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("checksum"));
}
