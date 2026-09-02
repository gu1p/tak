use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

pub(super) async fn accept_and_close_http2_probe(listener: &TcpListener) {
    let (mut stream, _) = listener.accept().await.expect("accept h2 probe");
    let mut buffer = [0_u8; 64];
    let _ = stream.read(&mut buffer).await;
}
