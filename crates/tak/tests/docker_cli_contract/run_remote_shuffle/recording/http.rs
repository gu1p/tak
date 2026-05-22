use std::io::{Read, Write};
use std::net::TcpStream;

use prost::Message;

pub(super) fn write_protobuf_response(
    stream: &mut TcpStream,
    status: &str,
    message: &impl Message,
) {
    let body = message.encode_to_vec();
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    let _ = stream.write_all(&body);
}

pub(super) fn read_request_head(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buf = [0_u8; 256];
    loop {
        let read = match stream.read(&mut buf) {
            Ok(read) => read,
            Err(_) => return String::new(),
        };
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buf[..read]);
        if let Some(index) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            request.truncate(index + 4);
            break;
        }
    }
    String::from_utf8(request).unwrap_or_default()
}
