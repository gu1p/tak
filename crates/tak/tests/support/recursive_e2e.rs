#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result, anyhow, bail};
use tempfile::{TempDir, tempdir, tempdir_in};

use super::e2e_harness::{find_free_tcp_port, write_tasks};
use super::recursive_e2e_docker::{
    docker_prereq_reason, run_tak_via_docker_container, spawn_containerized_remote_takd,
};

const TEST_ONION_HOST: &str =
    "abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion";
const DEFAULT_RUNNER_IMAGE: &str = "tak/e2e-runner:local";
const DEFAULT_BUILD_IMAGE: &str = "rust:1-bookworm";

pub struct RecursiveRunResult {
    pub stdout: String,
    pub workspace_root: PathBuf,
    _workspace_guard: TempDir,
}

pub fn skip_recursive_e2e_reason() -> Option<String> {
    if std::env::var("TAK_E2E_REAL_CONTAINER").ok().as_deref() != Some("1") {
        return Some("set TAK_E2E_REAL_CONTAINER=1".to_string());
    }
    if std::env::var("TAK_E2E_RECURSIVE_SELF_HOSTED")
        .ok()
        .as_deref()
        != Some("1")
    {
        return Some("set TAK_E2E_RECURSIVE_SELF_HOSTED=1".to_string());
    }
    docker_prereq_reason()
}

pub fn run_recursive_remote_task(task_name: &str, script: &str) -> Result<RecursiveRunResult> {
    if let Some(reason) = skip_recursive_e2e_reason() {
        bail!("recursive E2E prerequisites not satisfied: {reason}");
    }

    let repo_root = repo_root()?;
    let runner_image = std::env::var("TAK_E2E_RECURSIVE_RUNNER_IMAGE")
        .unwrap_or_else(|_| DEFAULT_RUNNER_IMAGE.to_string());
    ensure_runner_image(&runner_image)?;

    let workspace = tempdir_in(repo_root.join("target")).context("create recursive workspace")?;
    copy_repo_subset(&repo_root, workspace.path())?;
    let remote_port = find_free_tcp_port()?;
    let runtime_image = std::env::var("TAK_E2E_RECURSIVE_BUILD_IMAGE")
        .unwrap_or_else(|_| DEFAULT_BUILD_IMAGE.to_string());
    write_recursive_tasks(
        workspace.path(),
        remote_port,
        task_name,
        script,
        &runtime_image,
    )?;

    let service_root =
        tempdir_in(repo_root.join("target")).context("create recursive service root")?;
    let remote_takd = spawn_containerized_remote_takd(
        &runner_image,
        &repo_root,
        service_root.path(),
        remote_port,
    )?;

    let mut env = BTreeMap::new();
    env.insert(
        "TAK_TEST_TOR_ONION_DIAL_ADDR".to_string(),
        format!("host.docker.internal:{remote_port}"),
    );
    let events_wait_secs = std::env::var("TAK_E2E_RECURSIVE_EVENTS_WAIT_SECS")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "1800".to_string());
    env.insert(
        "TAK_REMOTE_EVENTS_MAX_WAIT_SECS".to_string(),
        events_wait_secs,
    );
    let output = run_tak_via_docker_container(
        &runner_image,
        &repo_root,
        workspace.path(),
        &["run", &format!("apps/web:{task_name}")],
        &env,
    )?;
    if !output.status.success() {
        let remote_logs = remote_takd.logs();
        let runtime_log = runtime_engine_log_snapshot(service_root.path());
        let submit_db_snapshot = remote_submit_db_snapshot(service_root.path());
        let exec_tree = remote_exec_tree_snapshot(&service_root.path().join("exec-root"))?;
        bail!(
            "recursive containerized tak run failed\nstdout:\n{}\nstderr:\n{}\nremote_takd_logs:\n{}\nruntime_engine_log_tail:\n{}\nremote_submit_db:\n{}\nremote_exec_root_tree:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            remote_logs,
            runtime_log,
            submit_db_snapshot,
            exec_tree
        );
    }

    Ok(RecursiveRunResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        workspace_root: workspace.path().to_path_buf(),
        _workspace_guard: workspace,
    })
}

fn remote_submit_db_snapshot(service_root: &Path) -> String {
    let db = service_root.join("remote-worker.sqlite");
    if !db.exists() {
        return format!("no remote submit DB at {}", db.display());
    }

    let query = r#"
SELECT 'submit_attempts' AS section, idempotency_key, task_run_id, attempt, selected_node_id, created_at_ms
FROM submit_attempts
ORDER BY created_at_ms DESC
LIMIT 5;
SELECT 'submit_results' AS section, idempotency_key, payload_json
FROM submit_results
ORDER BY rowid DESC
LIMIT 5;
SELECT 'submit_events' AS section, idempotency_key, seq, payload_json
FROM submit_events
ORDER BY rowid DESC
LIMIT 20;
"#;
    let output = std::process::Command::new("sqlite3")
        .arg(&db)
        .arg(query)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().is_empty() {
                    "<sqlite3 query succeeded with empty output>".to_string()
                } else {
                    stdout.to_string()
                }
            } else {
                format!(
                    "sqlite3 query failed (status={})\nstdout:\n{}\nstderr:\n{}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                )
            }
        }
        Err(error) => format!("failed to run sqlite3 query: {error}"),
    }
}

fn runtime_engine_log_snapshot(service_root: &Path) -> String {
    for candidate in ["podman-service.log", "dockerd.log"] {
        let path = service_root.join(candidate);
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let lines = content.lines().collect::<Vec<_>>();
        let tail = lines.len().saturating_sub(200);
        return lines[tail..].join("\n");
    }
    format!(
        "runtime engine logs not available under {}",
        service_root.display()
    )
}

fn remote_exec_tree_snapshot(root: &Path) -> Result<String> {
    if !root.exists() {
        return Ok(format!("remote exec root missing at {}", root.display()));
    }

    let mut entries = Vec::new();
    collect_tree_entries(root, root, 0, 4, &mut entries)?;
    if entries.is_empty() {
        return Ok("<remote exec root empty>".to_string());
    }
    Ok(entries.join("\n"))
}

fn collect_tree_entries(
    base: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    entries: &mut Vec<String>,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }

    let mut children = fs::read_dir(current)
        .with_context(|| format!("read directory {}", current.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("collect directory entries {}", current.display()))?;
    children.sort_unstable_by_key(|entry| entry.file_name());

    for child in children {
        let path = child.path();
        let relative = path.strip_prefix(base).with_context(|| {
            format!(
                "strip base prefix {} from {}",
                base.display(),
                path.display()
            )
        })?;
        let metadata =
            fs::metadata(&path).with_context(|| format!("read metadata for {}", path.display()))?;
        if metadata.is_dir() {
            entries.push(format!("{}/", relative.display()));
            collect_tree_entries(base, &path, depth + 1, max_depth, entries)?;
        } else {
            entries.push(format!("{} ({} bytes)", relative.display(), metadata.len()));
        }
        if entries.len() >= 200 {
            entries.push("... <truncated>".to_string());
            return Ok(());
        }
    }
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .context("resolve repository root")
}

fn container_cli_binary() -> Result<&'static str> {
    for candidate in ["docker", "podman"] {
        let status = std::process::Command::new(candidate)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if status.is_ok_and(|status| status.success()) {
            return Ok(candidate);
        }
    }
    bail!("no container engine CLI available (`docker` or `podman`)")
}

fn ensure_runner_image(image: &str) -> Result<()> {
    let cli = container_cli_binary()?;
    if runner_image_is_valid(cli, image) {
        return Ok(());
    }

    let context = tempdir().context("create docker build context")?;
    let mut build = std::process::Command::new(cli);
    build
        .current_dir(context.path())
        .args(["build", "-t", image, "-f", "-", "."])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let mut child = build
        .spawn()
        .with_context(|| format!("spawn {cli} build for runner image"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("docker build stdin missing"))?;
    stdin.write_all(
        br#"FROM rust:1-bookworm
RUN apt-get update \
 && apt-get install -y --no-install-recommends docker.io podman ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
"#,
    )?;
    drop(stdin);
    let status = child.wait().context("wait docker build runner image")?;
    if !status.success() {
        bail!("failed building recursive runner image `{image}`");
    }
    Ok(())
}

fn runner_image_is_valid(cli: &str, image: &str) -> bool {
    let has_image = std::process::Command::new(cli)
        .args(["image", "inspect", image])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if !has_image {
        return false;
    }
    std::process::Command::new(cli)
        .args([
            "run",
            "--rm",
            image,
            "sh",
            "-c",
            "command -v cargo >/dev/null && command -v docker >/dev/null && command -v podman >/dev/null",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn write_recursive_tasks(
    workspace: &Path,
    remote_port: u16,
    task_name: &str,
    script: &str,
    runtime_image: &str,
) -> Result<()> {
    let script_literal =
        serde_json::to_string(script).context("encode recursive script literal")?;
    write_tasks(
        workspace,
        &format!(
            r#"
REMOTE = Remote(
  id="remote-tor-recursive",
  transport=RemoteTransportMode.TorOnionService(endpoint="http://{TEST_ONION_HOST}:{remote_port}"),
  runtime=ContainerRuntime(image="{runtime_image}"),
)

SPEC = module_spec(tasks=[
  task(
    "{task_name}",
    steps=[cmd("sh", "-c", {script_literal})],
    execution=RemoteOnly(REMOTE),
  )
])
SPEC
"#
        ),
    )
}

fn copy_repo_subset(repo_root: &Path, dst: &Path) -> Result<()> {
    copy_path(&repo_root.join("Cargo.toml"), &dst.join("Cargo.toml"))?;
    copy_path(&repo_root.join("Cargo.lock"), &dst.join("Cargo.lock"))?;
    copy_path(&repo_root.join("crates"), &dst.join("crates"))
}

fn copy_path(src: &Path, dst: &Path) -> Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst).with_context(|| format!("create directory {}", dst.display()))?;
        for entry in
            fs::read_dir(src).with_context(|| format!("read directory {}", src.display()))?
        {
            let entry = entry.with_context(|| format!("iterate directory {}", src.display()))?;
            copy_path(&entry.path(), &dst.join(entry.file_name()))?;
        }
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create parent {}", parent.display()))?;
    }
    fs::copy(src, dst).with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    Ok(())
}
