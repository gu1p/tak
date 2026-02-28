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
    let remote_http_bind = remote_http_port.map(|port| format!("127.0.0.1:{port}"));
    spawn_daemon_with_remote_bind(socket_path, db_path, remote_http_bind, extra_env)
}

pub fn spawn_daemon_with_remote_bind(
    socket_path: PathBuf,
    db_path: PathBuf,
    remote_http_bind_addr: Option<String>,
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

    let remote_http_port = remote_http_bind_addr
        .as_deref()
        .map(parse_bind_addr_port)
        .transpose()?;
    if let Some(bind_addr) = remote_http_bind_addr.as_deref() {
        command.env("TAKD_REMOTE_V1_BIND_ADDR", bind_addr);
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

fn parse_bind_addr_port(bind_addr: &str) -> Result<u16> {
    let (_, port) = bind_addr
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("invalid remote HTTP bind addr: {bind_addr}"))?;
    port.parse::<u16>()
        .with_context(|| format!("invalid remote HTTP bind port in `{bind_addr}`"))
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

pub fn run_tak_in_container_expect_success(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
) -> Result<String> {
    let output = run_tak_in_container(workspace_root, args, extra_env)?;
    if !output.status.success() {
        bail!(
            "containerized command `tak {}` failed\nstdout:\n{}\nstderr:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn run_tak_in_container(
    workspace_root: &Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
) -> Result<Output> {
    let engine = detect_container_engine()
        .ok_or_else(|| anyhow!("no outer container engine available (`docker` or `podman`)"))?;
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .context("failed to resolve repository root")?;
    let workspace_root = workspace_root
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", workspace_root.display()))?;
    let image =
        std::env::var("TAK_E2E_CLIENT_CONTAINER_IMAGE").unwrap_or_else(|_| "rust:1".to_string());
    let cache_root = std::env::temp_dir().join("tak-e2e-client-cache");
    let cargo_home = cache_root.join("cargo-home");
    let cargo_target = cache_root.join("cargo-target");
    fs::create_dir_all(&cargo_home)
        .with_context(|| format!("failed to create {}", cargo_home.display()))?;
    fs::create_dir_all(&cargo_target)
        .with_context(|| format!("failed to create {}", cargo_target.display()))?;

    let mut command = StdCommand::new(engine);
    command.args(["run", "--rm"]);
    if engine == "docker" && cfg!(target_os = "linux") {
        command.args(["--add-host", "host.docker.internal:host-gateway"]);
    }
    command.args(["-v", &format!("{}:/repo:ro", repo_root.display())]);
    command.args(["-v", &format!("{}:/workspace", workspace_root.display())]);
    command.args(["-v", &format!("{}:/cargo-home", cargo_home.display())]);
    command.args(["-v", &format!("{}:/cargo-target", cargo_target.display())]);
    command.args(["-w", "/workspace"]);

    let mut env_overrides = extra_env.clone();
    if engine == "podman"
        && let Some(dial_addr) = env_overrides.get("TAK_TEST_TOR_ONION_DIAL_ADDR")
        && let Some(port) = dial_addr.strip_prefix("host.docker.internal:")
    {
        env_overrides.insert(
            "TAK_TEST_TOR_ONION_DIAL_ADDR".to_string(),
            format!("host.containers.internal:{port}"),
        );
    }
    env_overrides
        .entry("TAKD_SOCKET".to_string())
        .or_insert_with(|| "/workspace/.missing-takd.sock".to_string());
    env_overrides.insert("CARGO_HOME".to_string(), "/cargo-home".to_string());
    env_overrides.insert("CARGO_TARGET_DIR".to_string(), "/cargo-target".to_string());
    env_overrides.insert("CARGO_BUILD_JOBS".to_string(), "1".to_string());
    env_overrides.insert("RUSTFLAGS".to_string(), "-Cdebuginfo=0".to_string());
    for (key, value) in &env_overrides {
        command.args(["-e", &format!("{key}={value}")]);
    }

    command.arg(image);
    command.args([
        "cargo",
        "run",
        "--quiet",
        "-j",
        "1",
        "--manifest-path",
        "/repo/Cargo.toml",
        "-p",
        "tak",
        "--",
    ]);
    command.args(args);
    command.output().with_context(|| {
        format!(
            "failed running containerized `tak {}` via {}",
            args.join(" "),
            engine
        )
    })
}

fn detect_container_engine() -> Option<&'static str> {
    for engine in ["docker", "podman"] {
        let Ok(status) = StdCommand::new(engine)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        else {
            continue;
        };
        if status.success() {
            return Some(engine);
        }
    }
    None
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

pub fn detect_podman_socket() -> Option<String> {
    let output = StdCommand::new("podman")
        .args(["system", "connection", "list", "--format", "json"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let entries = serde_json::from_slice::<serde_json::Value>(&output.stdout).ok()?;
    let entries = entries.as_array()?;
    for entry in entries {
        let uri = entry.get("URI").and_then(serde_json::Value::as_str)?;
        let Some(path) = uri.strip_prefix("unix://") else {
            continue;
        };
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
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
