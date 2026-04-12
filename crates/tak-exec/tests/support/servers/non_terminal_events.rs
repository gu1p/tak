#![allow(dead_code)]

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use super::super::http::{read_request_path, write_protobuf_response};
use super::non_terminal_events_responses::{
    error, event_response, node_info, result_response, submit_response,
};

pub struct NonTerminalEventsServer {
    pub base_url: String,
    pub events_calls: Arc<AtomicUsize>,
    handle: Option<thread::JoinHandle<()>>,
    port: u16,
}

impl NonTerminalEventsServer {
    pub fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind event server");
        let port = listener.local_addr().expect("event listener addr").port();
        let events_calls = Arc::new(AtomicUsize::new(0));
        let calls = Arc::clone(&events_calls);
        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept event request");
                let Some(path) = read_request_path(&mut stream) else {
                    continue;
                };
                match path.as_str() {
                    "/__shutdown" => {
                        write_protobuf_response(&mut stream, "200 OK", &error("shutdown"));
                        break;
                    }
                    "/v1/node/info" => {
                        write_protobuf_response(&mut stream, "200 OK", &node_info(port))
                    }
                    "/v1/tasks/submit" => {
                        write_protobuf_response(&mut stream, "200 OK", &submit_response())
                    }
                    _ if path.contains("/events") => write_protobuf_response(
                        &mut stream,
                        "200 OK",
                        &event_response(calls.fetch_add(1, Ordering::SeqCst) + 1),
                    ),
                    _ if path.contains("/result") && calls.load(Ordering::SeqCst) >= 3 => {
                        write_protobuf_response(&mut stream, "200 OK", &result_response())
                    }
                    _ if path.contains("/result") => write_protobuf_response(
                        &mut stream,
                        "404 Not Found",
                        &error("result_not_ready"),
                    ),
                    _ => {
                        write_protobuf_response(&mut stream, "404 Not Found", &error("unexpected"))
                    }
                }
            }
        });
        Self {
            base_url: format!("http://127.0.0.1:{port}"),
            events_calls,
            handle: Some(handle),
            port,
        }
    }
}

impl Drop for NonTerminalEventsServer {
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
