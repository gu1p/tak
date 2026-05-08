use std::collections::BTreeMap;
use std::io::Write;
use std::net::TcpListener;
use std::thread;

use anyhow::Result;

use super::ps_status_payload::{active_job, node_status_payload};
use crate::support::{RemoteRecord, run_tak_output, write_remote_inventory};

#[test]
fn docker_ps_lists_remote_tak_containers_with_kind_and_source() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let base_url = format!("http://{addr}");
    write_remote_inventory(&config_root, &[remote_record(&base_url)])?;
    let server = thread::spawn({
        let base_url = base_url.clone();
        move || {
            let (mut stream, _) = listener.accept().expect("accept docker ps status request");
            let request = crate::support::remote_cli::read_request(&mut stream);
            assert!(request.starts_with("GET /v1/node/status HTTP/1.1\r\n"));
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
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());
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
    assert!(stdout.contains("source=dockerfile:docker/Dockerfile"));
    assert!(stdout.contains("command=sleep 30"), "stdout:\n{stdout}");
    server.join().expect("status server should exit");
    Ok(())
}

fn remote_record(base_url: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: "builder-a".into(),
        display_name: "builder-a".into(),
        base_url: base_url.into(),
        bearer_token: "secret".into(),
        pools: vec!["default".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
        enabled: true,
    }
}
