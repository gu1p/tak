use anyhow::{Context, Result, anyhow, bail};
use tokio::io::{AsyncRead, AsyncReadExt};

pub(super) async fn read_http_response<S>(stream: &mut S, base_url: &str) -> Result<(u16, Vec<u8>)>
where
    S: AsyncRead + Unpin + ?Sized,
{
    let mut response = Vec::new();
    let mut chunk = [0_u8; 1024];
    let header_end = loop {
        if let Some(index) = response.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
        let read = stream.read(&mut chunk).await.context("read node probe")?;
        if read == 0 {
            bail!("malformed HTTP response from {base_url}");
        }
        response.extend_from_slice(&chunk[..read]);
    };
    let head = String::from_utf8_lossy(&response[..header_end]);
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| anyhow!("invalid HTTP status from {base_url}"))?;
    let content_length = content_length(&head, base_url)?;
    let mut body = response[header_end..].to_vec();
    while body.len() < content_length {
        let read = stream
            .read(&mut chunk)
            .await
            .context("read node probe body")?;
        if read == 0 {
            bail!("truncated HTTP response body from {base_url}");
        }
        body.extend_from_slice(&chunk[..read]);
    }
    body.truncate(content_length);
    Ok((status, body))
}

fn content_length(head: &str, base_url: &str) -> Result<usize> {
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            return value
                .trim()
                .parse::<usize>()
                .with_context(|| format!("invalid HTTP content-length from {base_url}"));
        }
    }
    Ok(0)
}
