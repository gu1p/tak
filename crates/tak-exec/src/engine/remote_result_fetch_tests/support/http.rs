use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub(in super::super) fn spawn_http_server(
    listener: TcpListener,
    responses: Vec<(u16, Vec<u8>)>,
) -> tokio::task::JoinHandle<usize> {
    tokio::spawn(async move {
        let mut served = 0_usize;
        for (status, body) in responses {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let mut buf = vec![0_u8; 2048];
            let _ = stream.read(&mut buf).await;
            let head = format!(
                "HTTP/1.1 {status} {}\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                reason_phrase(status),
                body.len()
            );
            if stream.write_all(head.as_bytes()).await.is_err() {
                break;
            }
            if stream.write_all(&body).await.is_err() {
                break;
            }
            let _ = stream.flush().await;
            let _ = stream.shutdown().await;
            served += 1;
        }
        served
    })
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "Status",
    }
}
