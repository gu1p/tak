use super::*;

const HTTP2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

pub(super) struct ProtocolPrefix {
    pub(super) bytes: Vec<u8>,
    pub(super) is_http2: bool,
}

pub(super) async fn read_protocol_prefix<S>(stream: &mut S) -> Result<ProtocolPrefix>
where
    S: AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    while bytes.len() < HTTP2_PREFACE.len() {
        let mut byte = [0_u8; 1];
        let read = stream
            .read(&mut byte)
            .await
            .context("read protocol prefix")?;
        if read == 0 {
            break;
        }
        bytes.push(byte[0]);
        if !HTTP2_PREFACE.starts_with(&bytes) {
            return Ok(ProtocolPrefix {
                bytes,
                is_http2: false,
            });
        }
    }
    Ok(ProtocolPrefix {
        is_http2: bytes == HTTP2_PREFACE,
        bytes,
    })
}

pub(super) struct PrefixedIo<S> {
    prefix: std::io::Cursor<Vec<u8>>,
    inner: S,
}

impl<S> PrefixedIo<S> {
    pub(super) fn new(prefix: Vec<u8>, inner: S) -> Self {
        Self {
            prefix: std::io::Cursor::new(prefix),
            inner,
        }
    }
}

impl<S> AsyncRead for PrefixedIo<S>
where
    S: AsyncRead + Unpin,
{
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let position = usize::try_from(self.prefix.position()).unwrap_or(usize::MAX);
        let prefix = self.prefix.get_ref();
        if position < prefix.len() {
            let remaining = &prefix[position..];
            let len = remaining.len().min(buf.remaining());
            buf.put_slice(&remaining[..len]);
            self.prefix
                .set_position(u64::try_from(position + len).unwrap_or(u64::MAX));
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S> AsyncWrite for PrefixedIo<S>
where
    S: AsyncWrite + Unpin,
{
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}
