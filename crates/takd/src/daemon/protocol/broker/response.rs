use super::*;

#[derive(Clone)]
pub(super) struct BrokerHttpError {
    status: &'static str,
    code: &'static str,
    detail: String,
}

impl BrokerHttpError {
    pub(super) fn bad_request(code: &'static str) -> Self {
        Self {
            status: "400 Bad Request",
            code,
            detail: code.to_string(),
        }
    }

    pub(super) fn bad_request_with_source(
        code: &'static str,
        source: impl std::fmt::Display,
    ) -> Self {
        Self {
            status: "400 Bad Request",
            code,
            detail: format!("{code}: {source}"),
        }
    }

    pub(super) fn bad_gateway(code: &'static str, source: impl std::fmt::Display) -> Self {
        Self {
            status: "502 Bad Gateway",
            code,
            detail: format!("{code}: {source}"),
        }
    }

    pub(super) fn code(&self) -> &'static str {
        self.code
    }
}

impl From<BrokerHttpError> for anyhow::Error {
    fn from(value: BrokerHttpError) -> Self {
        anyhow!(value.detail)
    }
}

pub(super) async fn write_broker_error<W>(writer: &mut W, err: BrokerHttpError) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let body = err.detail.into_bytes();
    let head = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nX-Tak-Broker-Error: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        err.status,
        err.code,
        body.len()
    );
    writer.write_all(head.as_bytes()).await?;
    writer.write_all(&body).await?;
    writer.flush().await?;
    Ok(())
}
