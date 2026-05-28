#![allow(dead_code)]

use std::sync::{Arc, atomic::Ordering};
use std::time::Duration;

use tokio::net::TcpListener;

#[path = "http2_remote/server.rs"]
mod server;

use server::{Http2RemoteCounters, spawn_loop};

pub struct Http2Remote {
    pub addr: String,
    counters: Arc<Http2RemoteCounters>,
}

impl Http2Remote {
    pub async fn spawn(body: Vec<u8>) -> Self {
        Self::spawn_delayed(body, Duration::ZERO).await
    }

    pub async fn spawn_with_content_length(body: Vec<u8>, content_length: usize) -> Self {
        Self::spawn_with_options(body, Duration::ZERO, Some(content_length), 200).await
    }

    pub async fn spawn_status(status: u16, body: Vec<u8>) -> Self {
        Self::spawn_with_options(body, Duration::ZERO, None, status).await
    }

    #[rustfmt::skip]
    pub async fn spawn_delayed(body: Vec<u8>, response_delay: Duration) -> Self {
        Self::spawn_with_options(body, response_delay, None, 200).await
    }

    #[rustfmt::skip]
    async fn spawn_with_options(
        body: Vec<u8>,
        response_delay: Duration,
        content_length: Option<usize>,
        status: u16,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        let counters = Arc::new(Http2RemoteCounters::default());
        spawn_loop(
            listener,
            Arc::clone(&counters),
            Arc::new(body),
            response_delay,
            content_length,
            status,
        );
        Self { addr, counters }
    }

    pub fn connection_count(&self) -> usize {
        self.counters.connections.load(Ordering::SeqCst)
    }

    pub fn max_in_flight(&self) -> usize {
        self.counters.max_in_flight.load(Ordering::SeqCst)
    }
}
