use anyhow::Result;
use tokio::net::TcpStream;

use super::{
    BrokerClient, BrokerRemoteStream, TorBroker, socket_addr_from_host_port,
    tor_connect_retry_delay, tor_connect_timeout,
};

pub(super) async fn broker_connect(
    broker: &TorBroker,
    endpoint: &str,
) -> Result<BrokerRemoteStream> {
    let (host, port) = tak_core::endpoint::endpoint_host_port(endpoint)?;
    if !host.ends_with(".onion") {
        return connect_tcp(&socket_addr_from_host_port(&host, port)).await;
    }
    // Prefer the shared hidden-service client (one Arti client serves the onion
    // and dials peers). In `tor` transport that client is mandatory, so until it
    // is published we fail fast rather than bootstrapping a rival client.
    if let Some(shared) = broker.shared_tor_client_snapshot() {
        return retry_connect_arti(&shared, &host, port).await;
    }
    if broker.requires_shared_client() {
        anyhow::bail!("local hidden-service Tor client is not ready yet");
    }
    let client = broker.client().await?;
    retry_connect(client, &host, port).await
}

async fn retry_connect_arti(
    client: &arti_client::TorClient<tor_rtcompat::PreferredRuntime>,
    host: &str,
    port: u16,
) -> Result<BrokerRemoteStream> {
    let deadline = tokio::time::Instant::now() + tor_connect_timeout();
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        tracing::info!(onion = %host, port, attempt, "broker onion dial starting");
        let dial_started = tokio::time::Instant::now();
        match client.connect((host, port)).await {
            Ok(stream) => {
                tracing::info!(onion = %host, port, attempt, elapsed_ms = dial_started.elapsed().as_millis(), "broker onion dial succeeded");
                return Ok(Box::new(stream) as BrokerRemoteStream);
            }
            Err(err) if tokio::time::Instant::now() >= deadline => {
                tracing::warn!(onion = %host, port, attempt, error = %err, "broker onion dial failed: deadline reached");
                return Err(err.into());
            }
            Err(err) => {
                tracing::warn!(onion = %host, port, attempt, elapsed_ms = dial_started.elapsed().as_millis(), error = %err, "broker onion dial attempt failed; retrying");
                tokio::time::sleep(tor_connect_retry_delay()).await;
            }
        }
    }
}

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
