use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use bytes::Bytes;
use http_body_util::Full;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;

pub(super) fn spawn_loop(
    listener: TcpListener,
    counters: Arc<Http2RemoteCounters>,
    body: Arc<Vec<u8>>,
    response_delay: Duration,
    content_length: Option<usize>,
    status: u16,
) {
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            counters.connections.fetch_add(1, Ordering::SeqCst);
            tokio::spawn(serve_connection(
                stream,
                Arc::clone(&counters),
                Arc::clone(&body),
                response_delay,
                content_length,
                status,
            ));
        }
    });
}

async fn serve_connection(
    stream: tokio::net::TcpStream,
    counters: Arc<Http2RemoteCounters>,
    body: Arc<Vec<u8>>,
    response_delay: Duration,
    content_length: Option<usize>,
    status: u16,
) {
    let service = service_fn(move |_request: Request<hyper::body::Incoming>| {
        let counters = Arc::clone(&counters);
        let body = Arc::clone(&body);
        async move {
            let current = counters.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            counters.max_in_flight.fetch_max(current, Ordering::SeqCst);
            tokio::time::sleep(response_delay).await;
            counters.in_flight.fetch_sub(1, Ordering::SeqCst);
            let mut response = Response::builder()
                .status(status)
                .header("Content-Type", "application/x-protobuf");
            if let Some(content_length) = content_length {
                response = response.header("Content-Length", content_length);
            }
            Ok::<_, std::convert::Infallible>(
                response
                    .body(Full::new(Bytes::from(body.as_ref().clone())))
                    .expect("response"),
            )
        }
    });
    let _ = hyper::server::conn::http2::Builder::new(TokioExecutor::new())
        .serve_connection(TokioIo::new(stream), service)
        .await;
}

pub(super) struct Http2RemoteCounters {
    pub(super) connections: AtomicUsize,
    pub(super) in_flight: AtomicUsize,
    pub(super) max_in_flight: AtomicUsize,
}

impl Default for Http2RemoteCounters {
    fn default() -> Self {
        Self {
            connections: AtomicUsize::new(0),
            in_flight: AtomicUsize::new(0),
            max_in_flight: AtomicUsize::new(0),
        }
    }
}
