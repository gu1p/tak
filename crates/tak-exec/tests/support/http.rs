#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use prost::Message;

pub fn read_request_path(stream: &mut TcpStream) -> Option<String> {
    read_request_path_and_body(stream).map(|request| request.path)
}

pub fn read_request_path_and_body(stream: &mut TcpStream) -> Option<TestHttpRequest> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).ok()? == 0 {
        return None;
    }
    let mut request_fields = request_line.split_whitespace();
    let method = request_fields.next()?.to_string();
    let path = request_fields.next()?.to_string();
    let mut content_length = 0_usize;
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header).ok()? == 0 || matches!(header.as_str(), "\r\n" | "\n") {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.trim().eq_ignore_ascii_case("content-length")
        {
            content_length = value.trim().parse::<usize>().unwrap_or(0);
        }
    }
    let body = if content_length > 0 {
        let mut body = vec![0_u8; content_length];
        reader.read_exact(&mut body).ok()?;
        body
    } else {
        Vec::new()
    };
    Some(TestHttpRequest { method, path, body })
}

pub struct TestHttpRequest {
    pub method: String,
    pub path: String,
    pub body: Vec<u8>,
}

pub fn write_protobuf_response<M: Message>(stream: &mut TcpStream, status: &str, message: &M) {
    let body = message.encode_to_vec();
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
}
