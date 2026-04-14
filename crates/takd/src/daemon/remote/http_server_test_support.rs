use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use super::{RemoteNodeContext, SubmitAttemptStore};

pub(super) struct ScriptedHttpStream {
    read_bytes: Vec<u8>,
    read_offset: usize,
    pub(super) written_bytes: Vec<u8>,
    pub(super) flush_error: Option<io::ErrorKind>,
    pub(super) shutdown_error: Option<io::ErrorKind>,
}

impl ScriptedHttpStream {
    pub(super) fn with_request(request: &str) -> Self {
        Self {
            read_bytes: request.as_bytes().to_vec(),
            read_offset: 0,
            written_bytes: Vec::new(),
            flush_error: None,
            shutdown_error: None,
        }
    }
}

impl AsyncRead for ScriptedHttpStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.read_offset >= self.read_bytes.len() {
            return Poll::Ready(Ok(()));
        }
        let remaining = &self.read_bytes[self.read_offset..];
        let len = remaining.len().min(buf.remaining());
        buf.put_slice(&remaining[..len]);
        self.read_offset += len;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for ScriptedHttpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.written_bytes.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        if let Some(kind) = self.flush_error.take() {
            return Poll::Ready(Err(io::Error::new(kind, "scripted flush failure")));
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, _cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        if let Some(kind) = self.shutdown_error.take() {
            return Poll::Ready(Err(io::Error::new(kind, "scripted shutdown failure")));
        }
        Poll::Ready(Ok(()))
    }
}

pub(super) fn node_context() -> RemoteNodeContext {
    RemoteNodeContext::new(
        tak_proto::NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://builder-a.onion".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
    )
}

pub(super) fn request_bytes() -> &'static str {
    "GET /v1/node/info HTTP/1.1\r\nHost: builder-a.onion\r\nAuthorization: Bearer secret\r\nConnection: close\r\n\r\n"
}

pub(super) fn store() -> (tempfile::TempDir, SubmitAttemptStore) {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("takd.sqlite")).expect("store");
    (temp, store)
}
