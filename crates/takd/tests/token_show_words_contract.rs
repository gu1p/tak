use crate::support;

use std::fs;
use std::process::Command as StdCommand;

use tak_proto::{TOR_INVITE_WORD_COUNT, decode_tor_invite_words, encode_tor_invite};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn token_show_words_prints_nineteen_word_phrase_for_real_v3_invite() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode invite");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--words",
        ])
        .output()
        .expect("run takd token show --words");

    assert!(
        show.status.success(),
        "takd token show --words should succeed"
    );
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert_eq!(stdout.split_whitespace().count(), TOR_INVITE_WORD_COUNT);
    assert_eq!(
        decode_tor_invite_words(stdout.trim()).expect("decode tor invite words"),
        invite
    );
}

#[test]
fn token_show_words_rejects_non_v3_onion_invites() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let invite = encode_tor_invite("http://builder-a.onion").expect("encode short invite");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--words",
        ])
        .output()
        .expect("run takd token show --words");

    assert!(!show.status.success(), "non-v3 invite should fail");
    assert!(String::from_utf8_lossy(&show.stderr).contains("v3"));
}
