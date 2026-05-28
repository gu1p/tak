use anyhow::Result;
use tokio::net::TcpStream;

use super::{BrokerClient, BrokerRemoteStream, tor_connect_retry_delay, tor_connect_timeout};

pub(super) async fn retry_connect(
    client: &BrokerClient,
    host: &str,
    port: u16,
) -> Result<BrokerRemoteStream> {
    let deadline = tokio::time::Instant::now() + tor_connect_timeout();
    loop {
        let result: Result<BrokerRemoteStream> = match client {
            BrokerClient::Test(dial_addr) => TcpStream::connect(dial_addr)
                .await
                .map(|stream| Box::new(stream) as BrokerRemoteStream)
                .map_err(Into::into),
            BrokerClient::Arti(client) => client
                .connect((host, port))
                .await
                .map(|stream| Box::new(stream) as BrokerRemoteStream)
                .map_err(Into::into),
        };
        match result {
            Ok(stream) => return Ok(stream),
            Err(err) if tokio::time::Instant::now() >= deadline => return Err(err),
            Err(_) => tokio::time::sleep(tor_connect_retry_delay()).await,
        }
    }
}

pub(super) async fn connect_tcp(socket_addr: &str) -> Result<BrokerRemoteStream> {
    Ok(Box::new(TcpStream::connect(socket_addr).await?))
}
