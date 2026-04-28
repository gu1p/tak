use std::io;

use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;

pub(super) struct FakeDockerRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) body: Vec<u8>,
}

impl FakeDockerRequest {
    pub(super) fn path_without_query(&self) -> &str {
        self.path
            .split_once('?')
            .map_or(self.path.as_str(), |(path, _)| path)
    }

    pub(super) fn query_param(&self, key: &str) -> Option<String> {
        let (_, query) = self.path.split_once('?')?;
        url::form_urlencoded::parse(query.as_bytes())
            .find_map(|(name, value)| (name == key).then(|| value.into_owned()))
    }

    pub(super) fn requested_image_name(&self) -> Option<String> {
        let path = self.path_without_query();
        let tail = path.split("/images/").nth(1)?;
        let image = tail.strip_suffix("/json")?;
        Some(decode_image_reference(image))
    }

    pub(super) fn deleted_image_name(&self) -> Option<String> {
        let path = self.path_without_query();
        let image = path.split("/images/").nth(1)?;
        Some(decode_image_reference(image))
    }

    pub(super) fn pull_image_name(&self) -> Option<String> {
        self.query_param("fromImage").map(|image| image.to_string())
    }
}

fn decode_image_reference(image: &str) -> String {
    image
        .replace("%3A", ":")
        .replace("%2F", "/")
        .replace("%40", "@")
}

pub(super) async fn read_request(stream: &mut UnixStream) -> io::Result<FakeDockerRequest> {
    let mut buffer = Vec::new();
    let header_end = loop {
        if let Some(index) = buffer.windows(4).position(|window| window == b"\r\n\r\n") {
            break index;
        }
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "fake docker request ended before headers",
            ));
        }
        buffer.extend_from_slice(&chunk[..bytes_read]);
    };

    let headers = String::from_utf8_lossy(&buffer[..header_end]).to_string();
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("parse content-length"))
        })
        .unwrap_or(0);
    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let mut chunk = [0_u8; 1024];
        let bytes_read = stream.read(&mut chunk).await?;
        if bytes_read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "fake docker request ended before body",
            ));
        }
        buffer.extend_from_slice(&chunk[..bytes_read]);
    }

    let mut parts = headers
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace();
    Ok(FakeDockerRequest {
        method: parts.next().unwrap_or_default().to_string(),
        path: parts.next().unwrap_or_default().to_string(),
        body: buffer[body_start..body_start + content_length].to_vec(),
    })
}
