use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::thread;

use super::RecordingEvents;
use super::remote_routes::{SubmitBehavior, serve_remote_request};

pub struct RecordingRemoteServer {
    pub base_url: String,
    pub node_id: String,
    handle: Option<thread::JoinHandle<()>>,
    port: u16,
}

impl RecordingRemoteServer {
    pub fn spawn_success(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn(node_id, events, SubmitBehavior::Success)
    }

    pub fn spawn_submit_failure(node_id: &str, events: RecordingEvents) -> Self {
        Self::spawn(node_id, events, SubmitBehavior::Failure)
    }

    fn spawn(node_id: &str, events: RecordingEvents, submit: SubmitBehavior) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind recording remote server");
        let port = listener
            .local_addr()
            .expect("recording listener addr")
            .port();
        let node_id = node_id.to_string();
        let thread_node_id = node_id.clone();
        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept recording remote request");
                if !serve_remote_request(&mut stream, &thread_node_id, port, &events, submit) {
                    break;
                }
            }
        });
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            node_id,
            handle: Some(handle),
            port,
        }
    }
}

impl Drop for RecordingRemoteServer {
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
