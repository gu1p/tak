use std::collections::BTreeMap;
use std::io::Write;
use std::net::TcpListener;
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use prost::Message;
use tak_proto::{ActiveJob, CpuUsage, MemoryUsage, NodeInfo, NodeStatusResponse, StorageUsage};

use crate::support::{
    self, RemoteRecord, run_tak_expect_failure, run_tak_output, write_remote_inventory, write_tasks,
};
use support::direct_remote_runtime::{client_env, start_direct_agent};

#[test]
fn docker_build_is_rejected_with_tak_execution_guidance() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["docker", "build", "-t", "demo", "."],
        &BTreeMap::new(),
    )?;

    assert!(
        stderr.contains("tak docker build is not supported"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("tak docker run -f Dockerfile"),
        "stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn docker_run_rejects_detach() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["docker", "run", "--detach", "alpine:3.20", "true"],
        &BTreeMap::new(),
    )?;

    assert!(
        stderr.contains("tak docker run does not support detached containers"),
        "stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn docker_run_rejects_publish_until_forwarding_exists() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["docker", "run", "-p", "8080:80", "alpine:3.20", "true"],
        &BTreeMap::new(),
    )?;

    assert!(
        stderr.contains("tak docker run does not support port publishing yet"),
        "stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn docker_run_defaults_to_remote_and_reports_missing_inventory() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        config_root.display().to_string(),
    );

    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["docker", "run", "alpine:3.20", "true"], &env)?;

    assert!(
        stderr.contains("no configured remote agents match tak docker run"),
        "stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn docker_run_accepts_global_local_selector_before_docker() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let output = run_tak_output(
        temp.path(),
        &[
            "--local",
            "docker",
            "run",
            "--detach",
            "alpine:3.20",
            "true",
        ],
        &BTreeMap::new(),
    )?;

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("tak docker run does not support detached containers"),
        "stderr:\n{stderr}"
    );
    Ok(())
}

#[test]
fn remote_list_prints_generated_alias_for_node_selection() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    write_remote_inventory(
        &config_root,
        &[RemoteRecord {
            node_id: "builder-node-123456".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:12345".into(),
            bearer_token: "secret".into(),
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into(), "arch:arm64".into(), "os:linux".into()],
            transport: "direct".into(),
            enabled: true,
        }],
    )?;

    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        config_root.display().to_string(),
    );
    let output = run_tak_output(temp.path(), &["remote", "list"], &env)?;
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("alias="), "stdout:\n{stdout}");
    assert!(stdout.contains("builder-node-123456"), "stdout:\n{stdout}");
    Ok(())
}

#[test]
fn docker_run_node_selector_accepts_generated_alias() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    write_remote_inventory(
        &config_root,
        &[RemoteRecord {
            node_id: "builder-node-123456".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:12345".into(),
            bearer_token: "secret".into(),
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into(), "arch:arm64".into(), "os:linux".into()],
            transport: "direct".into(),
            enabled: true,
        }],
    )?;

    let alias = support::remote_inventory::stable_remote_alias_for_test("builder-node-123456");
    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        config_root.display().to_string(),
    );
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &[
            "--node",
            alias.as_str(),
            "--arch",
            "arm64",
            "--os",
            "linux",
            "docker",
            "run",
            "alpine:3.20",
            "true",
        ],
        &env,
    )?;

    assert!(
        !stderr.contains("no configured remote agents match tak docker run"),
        "selector should resolve the alias before failing later:\n{stderr}"
    );
    Ok(())
}

#[test]
fn docker_run_executes_image_command_on_remote_by_default() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let _agent = start_direct_agent(temp.path(), temp.path(), "docker-run-remote");

    let output = run_tak_output(
        temp.path(),
        &[
            "docker",
            "run",
            "alpine:3.20",
            "sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        &client_env(temp.path()),
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "image");
    Ok(())
}

#[test]
fn docker_ps_lists_remote_tak_containers_with_kind_and_source() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let base_url = format!("http://{addr}");
    write_remote_inventory(
        &config_root,
        &[RemoteRecord {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: base_url.clone(),
            bearer_token: "secret".into(),
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            enabled: true,
        }],
    )?;
    let server = thread::spawn({
        let base_url = base_url.clone();
        move || {
            let (mut stream, _) = listener.accept().expect("accept docker ps status request");
            let request = support::remote_cli::read_request(&mut stream);
            assert!(
                request.starts_with("GET /v1/node/status HTTP/1.1\r\n"),
                "unexpected request: {request}"
            );
            let body = node_status_payload(
                "builder-a",
                &base_url,
                vec![
                    active_job(
                        "//:docker-run",
                        "docker-run-1",
                        "docker-run",
                        "image:alpine:3.20",
                        "sleep 30",
                    ),
                    active_job(
                        "//apps/web:build",
                        "task-run-1",
                        "task",
                        "dockerfile:docker/Dockerfile",
                        "make build",
                    ),
                ],
            );
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .expect("write response head");
            stream.write_all(&body).expect("write response body");
        }
    });

    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        config_root.display().to_string(),
    );
    let output = run_tak_output(temp.path(), &["docker", "ps"], &env)?;

    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Tak Containers"), "stdout:\n{stdout}");
    assert!(stdout.contains("node=builder-a"), "stdout:\n{stdout}");
    assert!(stdout.contains("kind=docker-run"), "stdout:\n{stdout}");
    assert!(stdout.contains("kind=task"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("source=image:alpine:3.20"),
        "stdout:\n{stdout}"
    );
    assert!(
        stdout.contains("source=dockerfile:docker/Dockerfile"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("command=sleep 30"), "stdout:\n{stdout}");
    server.join().expect("status server should exit");
    Ok(())
}

#[test]
fn docker_ps_lists_active_local_docker_run_from_task_history() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut env = support::container_runtime::simulated_container_runtime_env(temp.path());
    env.insert(
        "XDG_STATE_HOME".to_string(),
        temp.path().join("state").display().to_string(),
    );
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        temp.path().join("config").display().to_string(),
    );
    let mut child = spawn_tak_child(
        temp.path(),
        &[
            "--local",
            "docker",
            "run",
            "alpine:3.20",
            "sh",
            "-c",
            "sleep 10",
        ],
        &env,
    )?;
    let _guard = ChildCleanup(&mut child);

    let stdout = wait_for_docker_ps(
        temp.path(),
        &["--local", "docker", "ps"],
        &env,
        "kind=docker-run",
    )?;
    assert!(stdout.contains("node=local"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("source=image:alpine:3.20"),
        "stdout:\n{stdout}"
    );
    assert!(stdout.contains("command=sh -c"), "stdout:\n{stdout}");
    Ok(())
}

#[test]
fn docker_ps_does_not_list_remote_task_history_as_local_container() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let _agent = start_direct_agent(temp.path(), &workspace_root, "remote-history-builder");
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task("check", steps=[cmd("sh", "-c", "sleep 10")])])
SPEC
"#,
    )?;

    let mut env = client_env(temp.path());
    env.insert(
        "XDG_STATE_HOME".to_string(),
        temp.path().join("state").display().to_string(),
    );
    let mut child = spawn_tak_child(
        &workspace_root,
        &[
            "run",
            "--remote",
            "--container-image",
            "alpine:3.20",
            "check",
        ],
        &env,
    )?;
    let _guard = ChildCleanup(&mut child);

    let _remote_stdout = wait_for_docker_ps(
        &workspace_root,
        &["docker", "ps"],
        &env,
        "node=remote-history-builder kind=task",
    )?;

    let local_output = run_tak_output(&workspace_root, &["--local", "docker", "ps"], &env)?;
    assert!(
        local_output.status.success(),
        "status: {:?}",
        local_output.status
    );
    let local_stdout = String::from_utf8_lossy(&local_output.stdout);
    assert!(
        !local_stdout.contains("node=local"),
        "remote task history must not be reported as a local container:\n{local_stdout}"
    );
    Ok(())
}

fn spawn_tak_child(
    workspace_root: &std::path::Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
) -> Result<Child> {
    let mut command = StdCommand::new(support::tak_bin());
    command
        .current_dir(workspace_root)
        .args(args)
        .env("TAKD_SOCKET", workspace_root.join(".missing-takd.sock"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        command.env(key, value);
    }
    Ok(command.spawn()?)
}

fn wait_for_docker_ps(
    workspace_root: &std::path::Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
    needle: &str,
) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut last_stdout = String::new();
    while Instant::now() < deadline {
        let output = run_tak_output(workspace_root, args, extra_env)?;
        assert!(output.status.success(), "status: {:?}", output.status);
        last_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if last_stdout.contains(needle) {
            return Ok(last_stdout);
        }
        thread::sleep(Duration::from_millis(100));
    }
    anyhow::bail!("timed out waiting for `{needle}` in docker ps output:\n{last_stdout}");
}

fn node_status_payload(node_id: &str, base_url: &str, active_jobs: Vec<ActiveJob>) -> Vec<u8> {
    NodeStatusResponse {
        node: Some(NodeInfo {
            node_id: node_id.into(),
            display_name: node_id.into(),
            base_url: base_url.into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        }),
        sampled_at_ms: 1_734_000_000_000,
        cpu: Some(CpuUsage {
            utilization_percent: Some(12.5),
            logical_cores: 8,
        }),
        memory: Some(MemoryUsage {
            used_bytes: 2_048,
            total_bytes: 8_192,
        }),
        storage: Some(StorageUsage {
            path: "/tmp/takd-remote-exec".into(),
            total_bytes: 10_000,
            available_bytes: 7_000,
            used_bytes: 3_000,
            tak_execution_bytes: 256,
        }),
        allocated_needs: vec![],
        active_jobs,
        image_cache: None,
    }
    .encode_to_vec()
}

fn active_job(
    task_label: &str,
    task_run_id: &str,
    origin: &str,
    runtime_source: &str,
    command: &str,
) -> ActiveJob {
    ActiveJob {
        task_run_id: task_run_id.into(),
        attempt: 1,
        task_label: task_label.into(),
        started_at_ms: 1_734_000_000_000,
        needs: vec![],
        execution_root_bytes: 256,
        runtime: Some("containerized".into()),
        origin: Some(origin.into()),
        runtime_source: Some(runtime_source.into()),
        command: Some(command.into()),
    }
}

struct ChildCleanup<'a>(&'a mut Child);

impl Drop for ChildCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
