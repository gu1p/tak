//! Test harness for driving a real hyper HTTP/2 client against the real
//! `handle_remote_v1_stream` server path, plus stream wrappers that mimic arti
//! `DataStream` behaviour (tiny chunked reads/writes and writes that only land
//! on an explicit flush). Used by `http2_roundtrip_tests`.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use bytes::Bytes;
use http_body_util::{BodyExt, Empty};
use hyper::Request;
use hyper_util::rt::{TokioExecutor, TokioIo};
use prost::Message;
use tak_proto::NodeInfo;
use tokio::io::{AsyncRead, AsyncWrite, DuplexStream, ReadBuf};

#[path = "http2_roundtrip_support/upload.rs"]
mod upload;
pub(super) use upload::drive_h2_workspace_stream;

/// Wraps a `DuplexStream` and caps how many bytes flow through each
/// `poll_read` / `poll_write`, approximating arti's cell-sized chunking.
pub(super) struct ThrottledStream {
    inner: DuplexStream,
    max_read: usize,
    max_write: usize,
}

impl ThrottledStream {
    pub(super) fn new(inner: DuplexStream, max_read: usize, max_write: usize) -> Self {
        Self {
            inner,
            max_read,
            max_write,
        }
    }
}

impl AsyncRead for ThrottledStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let cap = self.max_read.min(buf.remaining());
        if cap == 0 {
            return Pin::new(&mut self.inner).poll_read(cx, buf);
        }
        let mut scratch = vec![0_u8; cap];
        let mut small = ReadBuf::new(&mut scratch);
        match Pin::new(&mut self.inner).poll_read(cx, &mut small) {
            Poll::Ready(Ok(())) => {
                let filled = small.filled().to_vec();
                buf.put_slice(&filled);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

impl AsyncWrite for ThrottledStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let len = self.max_write.min(buf.len());
        Pin::new(&mut self.inner).poll_write(cx, &buf[..len])
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Buffers every write and only forwards it on an explicit `poll_flush`,
/// mimicking arti's "data is not sent until you flush" model.
pub(super) struct FlushGatedStream {
    inner: DuplexStream,
    pending: Vec<u8>,
}

impl FlushGatedStream {
    pub(super) fn new(inner: DuplexStream) -> Self {
        Self {
            inner,
            pending: Vec::new(),
        }
    }
}

impl AsyncRead for FlushGatedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl AsyncWrite for FlushGatedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.pending.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while !self.pending.is_empty() {
            let chunk = std::mem::take(&mut self.pending);
            match Pin::new(&mut self.inner).poll_write(cx, &chunk) {
                Poll::Ready(Ok(n)) => {
                    if n < chunk.len() {
                        self.pending = chunk[n..].to_vec();
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => {
                    self.pending = chunk;
                    return Poll::Pending;
                }
            }
        }
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

// Performs `GET /v1/node/info` over a real hyper HTTP/2 client on `client_io`
// and decodes the protobuf response.
pub(super) async fn drive_h2_node_info<S>(client_io: S) -> Result<NodeInfo, String>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (mut sender, connection) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(client_io))
            .await
            .map_err(|err| format!("handshake: {err}"))?;
    let conn = tokio::spawn(connection);

    let request = Request::builder()
        .uri("/v1/node/info")
        .header(hyper::header::HOST, "builder-a.onion")
        .header(hyper::header::AUTHORIZATION, "Bearer secret")
        .body(Empty::<Bytes>::new())
        .map_err(|err| format!("build request: {err}"))?;
    let response = sender
        .send_request(request)
        .await
        .map_err(|err| format!("send_request: {err}"))?;
    let body = response
        .into_body()
        .collect()
        .await
        .map_err(|err| format!("collect body: {err}"))?
        .to_bytes();
    conn.abort();
    NodeInfo::decode(body).map_err(|err| format!("decode: {err}"))
}
