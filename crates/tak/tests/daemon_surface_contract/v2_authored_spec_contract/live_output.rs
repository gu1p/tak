use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use tak_core::model::WorkspaceSpec;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{tak_bin, write_tasks};

#[test]
fn a_no_newline_output_chunk_is_flushed_while_the_daemon_job_is_running() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let gate = temp.path().canonicalize().unwrap().join("gate");
    let mut child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:stream", "--pass-env", "GATE"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("GATE", &gate)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = child.stdout.take().unwrap();
    let (streamed_tx, streamed_rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 128];
        let mut reported = false;
        loop {
            let read = stdout.read(&mut buffer).unwrap();
            if read == 0 {
                return output;
            }
            output.extend_from_slice(&buffer[..read]);
            if !reported && output.windows(MARKER.len()).any(|window| window == MARKER) {
                streamed_tx.send(()).unwrap();
                reported = true;
            }
        }
    });
    let streamed = streamed_rx.recv_timeout(Duration::from_secs(3)).is_ok();
    std::fs::write(&gate, b"open").unwrap();
    let status = child.wait().unwrap();
    let output = reader.join().unwrap();
    assert!(streamed, "output was buffered until job completion");
    assert!(status.success(), "{}", String::from_utf8_lossy(&output));
}

const MARKER: &[u8] = b"live-no-newline";

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "live-output".into(),
        root: root.into(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task("stream", steps=[cmd("/bin/sh", "-c", "printf live-no-newline; until test -e \"$GATE\"; do sleep 0.05; done")])])
SPEC
"#;
