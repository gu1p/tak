#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as StdCommand, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow, bail};

pub fn write_tasks(root: &Path, body: &str) -> Result<()> {
    fs::create_dir_all(root.join("apps/web"))
        .with_context(|| format!("failed to create apps/web under {}", root.display()))?;
    fs::write(root.join("apps/web/TASKS.py"), body)
        .with_context(|| format!("failed to write TASKS.py under {}", root.display()))?;
    Ok(())
}

pub fn find_free_tcp_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to allocate free TCP port")?;
    let port = listener
        .local_addr()
        .context("failed to read local listener address")?
        .port();
    Ok(port)
}

#[derive(Debug)]
pub struct DaemonHandle {
    child: Child,
    pub socket_path: PathBuf,
    pub remote_http_port: Option<u16>,
}

impl DaemonHandle {
    pub fn remote_endpoint(&self) -> Result<String> {
        let port = self
            .remote_http_port
            .ok_or_else(|| anyhow!("daemon has no remote HTTP endpoint"))?;
        Ok(format!("http://127.0.0.1:{port}"))
    }
}

impl Drop for DaemonHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn spawn_daemon(
    socket_path: PathBuf,
    db_path: PathBuf,
    remote_http_port: Option<u16>,
    extra_env: &BTreeMap<String, String>,
) -> Result<DaemonHandle> {
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create socket parent {}", parent.display()))?;
    }
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create db parent {}", parent.display()))?;
    }

    let mut command = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    command
        .args(["daemon", "start"])
        .env("TAKD_SOCKET", &socket_path)
        .env("TAKD_DB_PATH", &db_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if let Some(port) = remote_http_port {
        command.env("TAKD_REMOTE_V1_BIND_ADDR", format!("127.0.0.1:{port}"));
    }
    for (key, value) in extra_env {
        command.env(key, value);
    }

    let child = command
        .spawn()
        .context("failed to spawn `tak daemon start`")?;
    wait_for_socket(&socket_path, Duration::from_secs(10))?;
    if let Some(port) = remote_http_port {
        wait_for_http_ready(port, "/v1/node/status", Duration::from_secs(10))?;
    }

    Ok(DaemonHandle {
        child,
        socket_path,
        remote_http_port,
    })
}

pub fn run_tak(
    workspace_root: &Path,
    args: &[&str],
    daemon_socket: Option<&Path>,
    extra_env: &BTreeMap<String, String>,
) -> Result<Output> {
    let mut command = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    command.current_dir(workspace_root).args(args);
    let fallback_socket = workspace_root.join(".missing-takd.sock");
    let effective_socket = daemon_socket.unwrap_or(fallback_socket.as_path());
    command.env("TAKD_SOCKET", effective_socket);
    for (key, value) in extra_env {
        command.env(key, value);
    }
    command
        .output()
        .with_context(|| format!("failed running `tak {}`", args.join(" ")))
}

pub fn run_tak_expect_success(
    workspace_root: &Path,
    args: &[&str],
    daemon_socket: Option<&Path>,
    extra_env: &BTreeMap<String, String>,
) -> Result<String> {
    let output = run_tak(workspace_root, args, daemon_socket, extra_env)?;
    if !output.status.success() {
        bail!(
            "command `tak {}` failed\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn run_tak_expect_failure(
    workspace_root: &Path,
    args: &[&str],
    daemon_socket: Option<&Path>,
    extra_env: &BTreeMap<String, String>,
) -> Result<(String, String)> {
    let output = run_tak(workspace_root, args, daemon_socket, extra_env)?;
    if output.status.success() {
        bail!(
            "command `tak {}` unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok((
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

pub fn wait_for_socket(path: &Path, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    bail!("timed out waiting for daemon socket {}", path.display());
}

pub fn wait_for_http_ready(port: u16, path: &str, timeout: Duration) -> Result<()> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Ok((status_code, _)) = http_get(port, path)
            && status_code == 200
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    bail!("timed out waiting for remote HTTP endpoint 127.0.0.1:{port}{path}");
}

fn http_get(port: u16, path: &str) -> Result<(u16, String)> {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port))
        .with_context(|| format!("failed to connect to remote HTTP endpoint on port {port}"))?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .context("failed to write HTTP request")?;
    stream.flush().context("failed to flush HTTP request")?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .context("failed to read HTTP response")?;
    let response_text = String::from_utf8_lossy(&response);
    let (head, body) = response_text
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow!("malformed HTTP response"))?;
    let status_code = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("failed parsing HTTP status code"))?;

    Ok((status_code, body.to_string()))
}
