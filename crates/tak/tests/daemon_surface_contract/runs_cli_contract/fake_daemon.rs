use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;

#[path = "fake_daemon/read.rs"]
mod read;
#[path = "fake_daemon/response.rs"]
mod response;
#[path = "fake_daemon/server.rs"]
mod server;
#[path = "fake_daemon/write.rs"]
mod write;

pub(crate) enum Reply {
    Inactive(&'static str),
    SlowDripInactive(&'static str, Duration, usize),
    Legacy(&'static str),
    Raw(Vec<u8>),
    RawThenStall(Vec<u8>),
    Retryable(&'static str),
    SubmissionFlow,
    FailedSubmissionFlow,
    RetrySubmissionFlow,
    ManagementFlow,
    FailedAttachFlow,
    UnsafeOutputFlow,
    SymlinkChainOutputFlow,
    HugeOutputFlow,
    Close,
    Success,
}

pub(crate) struct FakeRunDaemon {
    socket_path: PathBuf,
    stop: Arc<AtomicBool>,
    requests: Arc<Mutex<Vec<Value>>>,
    thread: Option<JoinHandle<()>>,
}

impl FakeRunDaemon {
    pub(crate) fn spawn(socket_path: &Path, reply: Reply) -> Self {
        let listener = std::os::unix::net::UnixListener::bind(socket_path)
            .expect("bind fake run daemon socket");
        let stop = Arc::new(AtomicBool::new(false));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let thread_stop = Arc::clone(&stop);
        let thread_requests = Arc::clone(&requests);
        let thread = std::thread::spawn(move || {
            server::serve(listener, reply, &thread_stop, &thread_requests);
        });
        Self {
            socket_path: socket_path.to_path_buf(),
            stop,
            requests,
            thread: Some(thread),
        }
    }

    pub(crate) fn finish_expecting(mut self, expected: usize) -> Vec<Value> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.requests.lock().expect("request capture lock").len() < expected
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        self.stop_and_join();
        let requests = self.requests.lock().expect("request capture lock").clone();
        assert_eq!(
            requests.len(),
            expected,
            "unexpected fake-daemon request count: {requests:?}"
        );
        requests
    }

    fn stop_and_join(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = std::os::unix::net::UnixStream::connect(&self.socket_path);
        if let Some(thread) = self.thread.take() {
            thread.join().expect("join fake run daemon");
        }
    }
}

impl Drop for FakeRunDaemon {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}
