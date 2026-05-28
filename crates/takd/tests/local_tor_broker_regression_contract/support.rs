use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub(super) async fn spawn_remote_that_keeps_connection_open() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut request = [0_u8; 512];
        let _ = stream.read(&mut request).await.expect("read request");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok")
            .await
            .expect("write response");
        stream.flush().await.expect("flush response");
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    addr
}

pub(super) async fn spawn_remote_with_oversized_content_length() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut request = [0_u8; 512];
        let _ = stream.read(&mut request).await.expect("read request");
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 536870913\r\n\r\n")
            .await
            .expect("write response");
        stream.flush().await.expect("flush response");
    });
    addr
}

pub(super) async fn spawn_http2_remote_that_fails_after_request()
-> (String, Arc<Mutex<Vec<String>>>, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr").to_string();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let connection_count = Arc::new(AtomicUsize::new(0));
    let requests_for_task = Arc::clone(&requests);
    let connections_for_task = Arc::clone(&connection_count);
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            connections_for_task.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(serve_failing_http2_post(
                stream,
                Arc::clone(&requests_for_task),
            ));
        }
    });
    (addr, requests, connection_count)
}

async fn serve_failing_http2_post(
    stream: tokio::net::TcpStream,
    requests: Arc<Mutex<Vec<String>>>,
) {
    let service = hyper::service::service_fn(move |request: Request<hyper::body::Incoming>| {
        let requests = Arc::clone(&requests);
        async move {
            let method = request.method().to_string();
            let path = request.uri().path().to_string();
            let _ = request.into_body().collect().await;
            requests
                .lock()
                .expect("remote request lock")
                .push(format!("{method} {path}"));
            Err::<Response<Full<Bytes>>, std::io::Error>(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "fail after receiving request",
            ))
        }
    });
    let _ = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
        .serve_connection(TokioIo::new(stream), service)
        .await;
}
