use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

use base64::Engine;
use sha2::{Digest, Sha256};
use tak_core::model::{
    CurrentStateSpec, LimiterKey, QueueDef, RemoteSelectionSpec, RemoteSpec, RemoteTransportKind,
    ResolvedTask, RetryDef, StepDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};
use tak_exec::{RunOptions, run_tasks};

struct RemoteWorkerServer {
    port: u16,
    handle: Option<thread::JoinHandle<()>>,
}

impl RemoteWorkerServer {
    fn spawn(output_bytes: Vec<u8>) -> Self {
        let output_b64 = base64::engine::general_purpose::STANDARD.encode(&output_bytes);
        let digest = format!("sha256:{:x}", Sha256::digest(&output_bytes));
        let size = output_bytes.len();
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind remote worker server");
        let port = listener.local_addr().expect("server addr").port();
        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept");
                let (path, _) = read_http_request(&mut stream);
                if path == "/__shutdown" {
                    write_json(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                    break;
                }
                if path == "/v1/node/capabilities" {
                    write_json(
                        &mut stream,
                        "200 OK",
                        r#"{"compatible":true,"remote_worker":true}"#,
                    );
                    continue;
                }
                if path == "/v1/node/status" {
                    write_json(&mut stream, "200 OK", r#"{"healthy":true}"#);
                    continue;
                }
                if path == "/v1/tasks/submit" {
                    write_json(
                        &mut stream,
                        "200 OK",
                        r#"{"accepted":true,"execution_mode":"remote_worker"}"#,
                    );
                    continue;
                }
                if path.contains("/events") {
                    write_json(
                        &mut stream,
                        "200 OK",
                        r#"{"seq":1,"task_run_id":"id","type":"TASK_LOG_CHUNK","payload":{"kind":"TASK_LOG_CHUNK","chunk":"remote-log\n"}}
{"seq":2,"task_run_id":"id","type":"TASK_COMPLETED","payload":{"kind":"TASK_COMPLETED","success":true}}"#,
                    );
                    continue;
                }
                if path.contains("/result") {
                    write_json(
                        &mut stream,
                        "200 OK",
                        &format!(
                            r#"{{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[{{"path":"dist/out.txt","digest":"{digest}","size":{size}}}],"runtime":"containerized","runtime_engine":"docker"}}"#
                        ),
                    );
                    continue;
                }
                if path.contains("/outputs") {
                    write_json(
                        &mut stream,
                        "200 OK",
                        &format!(r#"{{"data_base64":"{output_b64}"}}"#),
                    );
                    continue;
                }
                write_json(&mut stream, "404 Not Found", r#"{"error":"not_found"}"#);
            }
        });
        Self {
            port,
            handle: Some(handle),
        }
    }

    fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for RemoteWorkerServer {
    fn drop(&mut self) {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", self.port)) {
            let _ = stream.write_all(
                b"GET /__shutdown HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
            );
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn read_http_request(stream: &mut TcpStream) -> (String, String) {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .expect("read request line");
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let mut content_length = 0_usize;
    loop {
        let mut header = String::new();
        reader.read_line(&mut header).expect("read header");
        if header == "\r\n" || header == "\n" || header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().unwrap_or(0);
        }
    }
    let mut body = vec![0_u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).expect("read request body");
    }
    (path, String::from_utf8_lossy(&body).to_string())
}

fn write_json(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response");
}

#[tokio::test]
async fn remote_worker_mode_skips_local_step_and_downloads_outputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let local_marker = temp.path().join("local-should-not-run.log");
    let synced_output = temp.path().join("dist/out.txt");
    let remote = RemoteWorkerServer::spawn(b"remote-output\n".to_vec());

    let label = TaskLabel {
        package: "//apps/web".to_string(),
        name: "remote_worker_download".to_string(),
    };
    let task = ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![StepDef::Cmd {
            argv: vec![
                "sh".to_string(),
                "-c".to_string(),
                format!("echo local >> '{}'; exit 99", local_marker.display()),
            ],
            cwd: None,
            env: BTreeMap::new(),
        }],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        execution: TaskExecutionSpec::RemoteOnly(RemoteSelectionSpec::Single(RemoteSpec {
            id: "remote-worker-node".to_string(),
            endpoint: Some(remote.endpoint()),
            transport_kind: RemoteTransportKind::DirectHttps,
            service_auth_env: None,
            runtime: None,
        })),
        tags: Vec::new(),
    };

    let mut tasks = std::collections::BTreeMap::new();
    tasks.insert(label.clone(), task);
    let spec = WorkspaceSpec {
        project_id: "project-test".to_string(),
        root: temp.path().to_path_buf(),
        tasks,
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    };

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert!(
        !local_marker.exists(),
        "remote worker mode should not execute local steps"
    );
    assert_eq!(
        fs::read_to_string(&synced_output).expect("synced output file"),
        "remote-output\n"
    );
    assert_eq!(result.remote_runtime_kind.as_deref(), Some("containerized"));
    assert_eq!(result.remote_runtime_engine.as_deref(), Some("docker"));
}
