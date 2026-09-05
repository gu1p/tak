#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};

use anyhow::{Context, Result};

use super::tak_bin;

pub fn run_tak_terminal(
    root: &Path,
    arguments: &[&str],
    environment: &BTreeMap<String, String>,
) -> Result<Output> {
    let mut command = terminal_command(arguments);
    command.current_dir(root).env("TERM", "xterm-256color");
    for (name, value) in environment {
        command.env(name, value);
    }
    command.output().context("run tak in a pseudo-terminal")
}

pub fn spawn_tak_terminal(
    root: &Path,
    arguments: &[&str],
    environment: &BTreeMap<String, String>,
) -> Result<Child> {
    let mut command = terminal_command(arguments);
    command
        .current_dir(root)
        .env("TERM", "xterm-256color")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in environment {
        command.env(name, value);
    }
    command.spawn().context("spawn tak in a pseudo-terminal")
}

pub fn send_terminal_input(child: &mut Child, input: &[u8]) -> Result<()> {
    let stdin = child.stdin.as_mut().context("pseudo-terminal stdin")?;
    stdin.write_all(input).context("write terminal input")?;
    stdin.flush().context("flush terminal input")?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn terminal_command(arguments: &[&str]) -> Command {
    let mut command = Command::new("script");
    command
        .args(["-q", "-e", "/dev/null"])
        .arg(tak_bin())
        .args(arguments);
    command
}

#[cfg(not(target_os = "macos"))]
fn terminal_command(arguments: &[&str]) -> Command {
    let invocation = std::iter::once(tak_bin().display().to_string())
        .chain(arguments.iter().map(|argument| (*argument).to_owned()))
        .map(|word| format!("'{}'", word.replace('\'', "'\\''")))
        .collect::<Vec<_>>()
        .join(" ");
    let mut command = Command::new("script");
    command.args(["-q", "-e", "-c", &invocation, "/dev/null"]);
    command
}
