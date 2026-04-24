use crate::support;

use std::fs;
use std::process::Command as StdCommand;

use tak_proto::{encode_tor_invite, encode_tor_invite_words};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[test]
fn token_show_words_table_renders_numbered_cells_for_manual_copying() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode invite");
    let words = encode_tor_invite_words(&invite).expect("encode invite words");
    fs::write(state_root.join("agent.token"), format!("{invite}\n")).expect("write invite");

    let show = StdCommand::new(support::takd_bin())
        .args([
            "token",
            "show",
            "--state-root",
            &state_root.display().to_string(),
            "--words-table",
        ])
        .output()
        .expect("run takd token show --words-table");

    assert!(
        show.status.success(),
        "takd token show --words-table should succeed"
    );
    let stdout = String::from_utf8_lossy(&show.stdout);
    assert!(stdout.contains("Words"), "missing words title:\n{stdout}");
    assert!(
        stdout.contains("01") && stdout.contains("19"),
        "missing cell numbers:\n{stdout}"
    );
    for word in words.split_whitespace() {
        assert!(stdout.contains(word), "missing word `{word}`:\n{stdout}");
    }
}
