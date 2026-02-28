//! Contract test for `tak run` delegation to daemon `RunTasks` request.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::process::Command as StdCommand;
use std::thread;

use takd::{Request, Response, RunTasksRequest};

fn write_tasks(root: &Path, body: &str) {
    fs::create_dir_all(root.join("apps/web")).expect("mkdir");
    fs::write(root.join("apps/web/TASKS.py"), body).expect("write tasks");
}

#[test]
fn run_command_prefers_daemon_run_tasks_protocol_when_available() {
    let temp = tempfile::tempdir().expect("tempdir");
    let marker = temp.path().join("direct-run-should-not-happen.log");
    write_tasks(
        temp.path(),
        &format!(
            r#"
SPEC = module_spec(tasks=[
  task("from_daemon", steps=[cmd("sh", "-c", "echo direct_run >> {marker}; exit 99")]),
])
SPEC
"#,
            marker = marker.display()
        ),
    );

    let socket = temp.path().join("takd.sock");
    let listener = UnixListener::bind(&socket).expect("bind fake daemon socket");
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept run request");
        let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
        let mut line = String::new();
        reader.read_line(&mut line).expect("read request");
        let request: Request = serde_json::from_str(line.trim_end()).expect("decode request");
        let Request::RunTasks(RunTasksRequest {
            request_id, labels, ..
        }) = request
        else {
            panic!("expected RunTasks request");
        };
        assert_eq!(labels, vec!["apps/web:from_daemon"]);

        let mut writer = stream;
        let started = serde_json::to_string(&Response::RunStarted {
            request_id: request_id.clone(),
        })
        .expect("encode started");
        writer
            .write_all(format!("{started}\n").as_bytes())
            .expect("write started");

        let result = serde_json::to_string(&Response::RunTaskResult {
            request_id: request_id.clone(),
            label: "apps/web:from_daemon".to_string(),
            attempts: 1,
            success: true,
            exit_code: Some(0),
            placement: "local".to_string(),
            remote_node: None,
            transport: None,
            reason: None,
            context_hash: None,
            runtime: None,
            runtime_engine: None,
        })
        .expect("encode task result");
        writer
            .write_all(format!("{result}\n").as_bytes())
            .expect("write task result");

        let completed = serde_json::to_string(&Response::RunCompleted { request_id })
            .expect("encode completed");
        writer
            .write_all(format!("{completed}\n").as_bytes())
            .expect("write completed");
        writer.flush().expect("flush responses");
    });

    let mut cmd = StdCommand::new(assert_cmd::cargo::cargo_bin!("tak"));
    cmd.current_dir(temp.path())
        .env("TAKD_SOCKET", &socket)
        .args(["run", "apps/web:from_daemon"]);
    let output = cmd.output().expect("run command should execute");
    assert!(
        output.status.success(),
        "run should succeed via daemon protocol\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("apps/web:from_daemon: ok"),
        "daemon task result should be rendered in run summary: {stdout}"
    );
    assert!(
        !marker.exists(),
        "direct in-process execution should not run when daemon protocol is available"
    );

    server
        .join()
        .expect("fake daemon thread should exit cleanly");
}
