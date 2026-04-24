#![allow(dead_code)]

use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Command as StdCommand, Output, Stdio};
use std::thread;

use anyhow::Result;
use prost::Message;
use tak_proto::{encode_tor_invite, encode_tor_invite_words};

use super::remote_cli::{node_info, read_request};

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

pub fn spawn_node_info_probe(
    listener: TcpListener,
    node_id: &'static str,
    base_url: String,
    transport: &'static str,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        assert!(read_request(&mut stream).starts_with("GET /v1/node/info HTTP/1.1\r\n"));
        let body = node_info(node_id, &base_url, transport).encode_to_vec();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .expect("write response");
        stream.write_all(&body).expect("write protobuf body");
    })
}
