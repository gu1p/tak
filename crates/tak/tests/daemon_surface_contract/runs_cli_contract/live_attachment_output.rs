use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use base64::Engine;
use serde_json::json;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::tak_bin;

#[test]
fn attach_flushes_a_no_newline_output_chunk_while_the_run_is_live() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let chunk = base64::engine::general_purpose::STANDARD.encode(MARKER);
    let mut response = serde_json::to_vec(&json!({
        "protocol_version": 2, "type": "RunEvents", "request_id": "tak-runs-attach",
        "run_id": "run-1", "next_event": 1, "state": "running", "terminal": false,
        "events": [{"seq": 1, "kind": "stdout", "job_id": "job-0",
            "task_ids": ["//:check"], "node_id": "local", "message": "",
            "chunk_base64": chunk}],
    }))
    .unwrap();
    response.push(b'\n');
    let daemon = FakeRunDaemon::spawn(&socket, Reply::RawThenStall(response));
    let mut child = Command::new(tak_bin())
        .args(["runs", "attach", "run-1"])
        .env("TAKD_SOCKET", &socket)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 128];
        loop {
            let read = stdout.read(&mut buffer).unwrap();
            if read == 0 {
                return output;
            }
            output.extend_from_slice(&buffer[..read]);
            if output.windows(MARKER.len()).any(|window| window == MARKER) {
                let _ = sender.send(());
            }
        }
    });
    let streamed = receiver.recv_timeout(Duration::from_secs(3)).is_ok();
    let live = child.try_wait().unwrap().is_none();
    if live {
        child.kill().unwrap();
    }
    child.wait().unwrap();
    let output = reader.join().unwrap();
    daemon.finish_expecting(1);
    assert!(streamed, "attach buffered persisted output");
    assert!(live, "attach exited before the live output was observed");
    assert!(output.windows(MARKER.len()).any(|window| window == MARKER));
}

const MARKER: &[u8] = b"live-attach-no-newline";
