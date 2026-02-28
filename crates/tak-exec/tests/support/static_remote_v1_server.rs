use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

pub struct StaticRemoteServer {
    port: u16,
    handle: Option<thread::JoinHandle<()>>,
}

impl StaticRemoteServer {
    pub fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind static remote");
        let port = listener.local_addr().expect("listener addr").port();
        let handle = thread::spawn(move || {
            loop {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut request_line = String::new();
                BufReader::new(stream.try_clone().expect("clone stream"))
                    .read_line(&mut request_line)
                    .expect("read request line");
                let path = request_line.split_whitespace().nth(1).unwrap_or("/");
                match path {
                    "/__shutdown" => {
                        write_json(&mut stream, "200 OK", r#"{"shutdown":true}"#);
                        break;
                    }
                    "/v1/node/capabilities" => {
                        write_json(&mut stream, "200 OK", r#"{"compatible":true}"#)
                    }
                    "/v1/node/status" => write_json(&mut stream, "200 OK", r#"{"healthy":true}"#),
                    "/v1/tasks/submit" => write_json(&mut stream, "200 OK", r#"{"accepted":true}"#),
                    _ if path.contains("/events") => {
                        write_json(&mut stream, "200 OK", r#"{"events":[],"done":true}"#)
                    }
                    _ if path.contains("/result") => write_json(
                        &mut stream,
                        "200 OK",
                        r#"{"success":true,"exit_code":0,"sync_mode":"OUTPUTS_AND_LOGS","outputs":[]}"#,
                    ),
                    _ => write_json(&mut stream, "404 Not Found", r#"{"error":"not_found"}"#),
                }
            }
        });
        Self {
            port,
            handle: Some(handle),
        }
    }

    pub fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for StaticRemoteServer {
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

fn write_json(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response");
}
