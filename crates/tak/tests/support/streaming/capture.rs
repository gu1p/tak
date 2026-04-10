use std::io::{BufRead, BufReader, Read};
use std::process::Command as StdCommand;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, bail};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StreamKind {
    Stdout,
    Stderr,
}

#[derive(Debug)]
struct StreamNotification {
    stream: StreamKind,
    line: String,
}

pub fn run_streaming_process_and_capture(mut command: StdCommand) -> Result<(String, String)> {
    let mut child = command.spawn().context("spawn tak command")?;
    let stdout = child.stdout.take().context("missing child stdout pipe")?;
    let stderr = child.stderr.take().context("missing child stderr pipe")?;

    let (notify_tx, notify_rx) = mpsc::channel();
    let stdout_reader = spawn_stream_reader(stdout, StreamKind::Stdout, notify_tx.clone());
    let stderr_reader = spawn_stream_reader(stderr, StreamKind::Stderr, notify_tx);

    wait_for_both_streams(&notify_rx)?;
    assert!(
        child.try_wait()?.is_none(),
        "tak process exited before streaming contract was observed"
    );

    let status = child.wait().context("wait for tak process")?;
    let stdout = stdout_reader.join().expect("join stdout reader");
    let stderr = stderr_reader.join().expect("join stderr reader");
    if !status.success() {
        bail!("tak process failed\nstdout:\n{stdout}\nstderr:\n{stderr}");
    }

    Ok((stdout, stderr))
}

fn spawn_stream_reader<R: Read + Send + 'static>(
    reader: R,
    stream: StreamKind,
    notify_tx: mpsc::Sender<StreamNotification>,
) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut full = String::new();
        let mut reader = BufReader::new(reader);
        let mut first_line_sent = false;
        loop {
            let mut line = String::new();
            let read = reader.read_line(&mut line).expect("read child stream line");
            if read == 0 {
                break;
            }
            if !first_line_sent {
                let _ = notify_tx.send(StreamNotification {
                    stream,
                    line: line.clone(),
                });
                first_line_sent = true;
            }
            full.push_str(&line);
        }
        full
    })
}

fn wait_for_both_streams(notify_rx: &mpsc::Receiver<StreamNotification>) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let mut saw_stdout = false;
    let mut saw_stderr = false;

    while !saw_stdout || !saw_stderr {
        let now = std::time::Instant::now();
        if now >= deadline {
            bail!("timed out waiting for stdout/stderr streaming lines");
        }
        let message = notify_rx
            .recv_timeout(deadline.saturating_duration_since(now))
            .context("wait for streaming lines")?;
        assert!(
            !message.line.is_empty(),
            "{:?} line should not be empty",
            message.stream
        );
        match message.stream {
            StreamKind::Stdout => saw_stdout = true,
            StreamKind::Stderr => saw_stderr = true,
        }
    }

    Ok(())
}
