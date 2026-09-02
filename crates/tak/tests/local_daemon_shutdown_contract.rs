use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Barrier, mpsc};
use std::time::{Duration, Instant};

use crate::support::local_daemon::LocalDaemonGuard;
use tak_core::model::WorkspaceSpec;

#[test]
fn repeated_in_process_daemon_shutdown_is_bounded() {
    let (finished_tx, finished_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        for index in 0..31 {
            let temp = tempfile::tempdir().expect("tempdir");
            let socket = temp.path().join("takd.sock");
            let guard = LocalDaemonGuard::spawn(&socket, &empty_spec(temp.path(), index));
            list_runs(guard.effective_socket_path(), index);
            drop(guard);
        }
        let _ = finished_tx.send(());
    });

    finished_rx
        .recv_timeout(Duration::from_secs(60))
        .expect("repeated daemon shutdown should not leak runtimes");
    worker.join().expect("daemon shutdown stress worker");
}

#[test]
fn parallel_in_process_daemon_shutdown_is_bounded() {
    const WORKERS: usize = 4;
    const CYCLES: usize = 8;
    let barrier = Arc::new(Barrier::new(WORKERS));
    let (finished_tx, finished_rx) = mpsc::channel();
    let workers = (0..WORKERS)
        .map(|worker| {
            let barrier = Arc::clone(&barrier);
            let finished_tx = finished_tx.clone();
            std::thread::spawn(move || {
                barrier.wait();
                for cycle in 0..CYCLES {
                    let index = 100 + worker * CYCLES + cycle;
                    let temp = tempfile::tempdir().expect("tempdir");
                    let socket = temp.path().join("takd.sock");
                    let guard = LocalDaemonGuard::spawn(&socket, &empty_spec(temp.path(), index));
                    list_runs(guard.effective_socket_path(), index);
                    drop(guard);
                }
                let _ = finished_tx.send(());
            })
        })
        .collect::<Vec<_>>();
    drop(finished_tx);

    let deadline = Instant::now() + Duration::from_secs(60);
    for _ in 0..WORKERS {
        finished_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("parallel daemon shutdown should remain isolated and bounded");
    }
    for worker in workers {
        worker.join().expect("parallel daemon shutdown worker");
    }
}

fn list_runs(socket: &Path, index: usize) {
    let mut stream = UnixStream::connect(socket).expect("connect daemon");
    stream
        .write_all(
            format!(
                "{{\"protocol_version\":2,\"request_id\":\"shutdown-{index}\",\"operation\":{{\"type\":\"ListRuns\"}}}}\n"
            )
            .as_bytes(),
        )
        .expect("write list request");
    stream.shutdown(Shutdown::Write).expect("finish request");
    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .expect("read list response");
    assert!(
        response.contains("RunList"),
        "unexpected response: {response}"
    );
}

fn empty_spec(root: &Path, index: usize) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: format!("shutdown-{index}"),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}
