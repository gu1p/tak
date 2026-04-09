use anyhow::{Context, Result, anyhow, bail};
use prost::Message;
use tak_proto::NodeInfo;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub async fn fetch_node_info<S>(
    stream: &mut S,
    authority: &str,
    bearer_token: &str,
    base_url: &str,
) -> Result<NodeInfo>
where
    S: AsyncRead + AsyncWrite + Unpin + ?Sized,
{
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {authority}\r\nAuthorization: Bearer {bearer_token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .context("write onion node request")?;
    stream.flush().await.context("flush onion node request")?;
    let (status, body) = read_http_response(stream, base_url).await?;
    if status != 200 {
        bail!("node probe failed with HTTP {status}");
    }
    NodeInfo::decode(body.as_slice()).context("decode onion node info")
}

async fn read_http_response<S>(stream: &mut S, base_url: &str) -> Result<(u16, Vec<u8>)>
where
    S: AsyncRead + Unpin + ?Sized,
{
    let mut response = Vec::new();
    let mut chunk = [0_u8; 1024];
    let split = loop {
        if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        let read = stream
            .read(&mut chunk)
            .await
            .context("read onion response")?;
        if read == 0 {
            bail!("malformed HTTP response from {base_url}");
        }
        response.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&response[..split]);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("invalid HTTP status from {base_url}"))?;
    let content_length = head
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim())
        })
        .map(str::parse::<usize>)
        .transpose()
        .with_context(|| format!("invalid HTTP content-length from {base_url}"))?
        .unwrap_or(0);
    let mut body = response[split..].to_vec();
    while body.len() < content_length {
        let read = stream
            .read(&mut chunk)
            .await
            .context("read onion response body")?;
        if read == 0 {
            bail!("truncated HTTP response body from {base_url}");
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    Ok((status, body))
}
