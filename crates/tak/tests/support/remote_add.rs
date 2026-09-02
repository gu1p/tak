#![allow(dead_code)]

use std::path::Path;
use std::process::{Command as StdCommand, Output, Stdio};

use anyhow::Result;
use tak_proto::{encode_tor_invite, encode_tor_invite_words};

const V3_BASE_URL: &str = "http://pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

pub fn run_add_script(config_root: &Path, script: &str, envs: &[(&str, String)]) -> Result<Output> {
    run_add_with_args(config_root, &["remote", "add"], script, envs)
}

pub fn run_add_words_script(
    config_root: &Path,
    script: &str,
    envs: &[(&str, String)],
) -> Result<Output> {
    run_add_with_args(config_root, &["remote", "add", "--words"], script, envs)
}

fn run_add_with_args(
    config_root: &Path,
    args: &[&str],
    script: &str,
    envs: &[(&str, String)],
) -> Result<Output> {
    let mut command = StdCommand::new(super::tak_bin());
    command
        .args(args)
        .env("XDG_CONFIG_HOME", config_root)
        .env("TAK_TEST_REMOTE_ADD_SCRIPT", script)
        .stdin(Stdio::null());
    for (key, value) in envs {
        command.env(key, value);
    }
    Ok(command.output()?)
}

pub fn tor_words() -> String {
    let invite = encode_tor_invite(V3_BASE_URL).expect("encode tor invite");
    encode_tor_invite_words(&invite).expect("encode tor invite words")
}
