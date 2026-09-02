use std::net::SocketAddr;

use prost::Message;
use tak_proto::ErrorResponse;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct RawHttpResponse {
    pub head: String,
    pub body: Vec<u8>,
}

pub async fn send_raw_request(addr: SocketAddr, request: &[u8]) -> RawHttpResponse {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect server");
    stream.write_all(request).await.expect("write request");
    stream.shutdown().await.expect("shutdown write side");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read response");
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .expect("response should contain HTTP header terminator");
    RawHttpResponse {
        head: String::from_utf8(response[..split].to_vec()).expect("response utf8"),
        body: response[split..].to_vec(),
    }
}

pub fn decode_error_response(response: &RawHttpResponse) -> ErrorResponse {
    ErrorResponse::decode(response.body.as_slice()).expect("decode error payload")
}
