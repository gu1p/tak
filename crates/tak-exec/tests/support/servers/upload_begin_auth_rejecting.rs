#![allow(dead_code)]

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::thread;

use super::super::http::{read_request_path, write_protobuf_response};
use super::auth_rejecting_submit_responses::{auth_failed, node_info, not_found, shutdown};

pub struct UploadBeginAuthRejectingServer {
    pub base_url: String,
    begin_requests: Arc<AtomicUsize>,
    request_paths: Arc<Mutex<Vec<String>>>,
    handle: Option<thread::JoinHandle<()>>,
    port: u16,
}

impl UploadBeginAuthRejectingServer {
    pub fn spawn(node_id: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind auth server");
        let port = listener.local_addr().expect("auth listener addr").port();
        let begin_requests = Arc::new(AtomicUsize::new(0));
        let request_paths = Arc::new(Mutex::new(Vec::new()));
        let begin_count = Arc::clone(&begin_requests);
        let paths = Arc::clone(&request_paths);
        let node_id = node_id.to_string();
        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept auth request");
                let Some(path) = read_request_path(&mut stream) else {
                    continue;
                };
                if path == "/__shutdown" {
                    write_protobuf_response(&mut stream, "200 OK", &shutdown());
                    break;
                }
                paths.lock().expect("lock request paths").push(path.clone());
                match path.as_str() {
                    "/v1/node/info" => {
                        write_protobuf_response(&mut stream, "200 OK", &node_info(&node_id, port))
                    }
                    "/v2/workspaces/uploads/begin" => {
                        begin_count.fetch_add(1, Ordering::SeqCst);
                        write_protobuf_response(&mut stream, "401 Unauthorized", &auth_failed())
                    }
                    _ => write_protobuf_response(&mut stream, "404 Not Found", &not_found(&path)),
                }
            }
        });
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            begin_requests,
            request_paths,
            handle: Some(handle),
            port,
        }
    }

    pub fn begin_requests(&self) -> usize {
        self.begin_requests.load(Ordering::SeqCst)
    }
}

impl Drop for UploadBeginAuthRejectingServer {
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
