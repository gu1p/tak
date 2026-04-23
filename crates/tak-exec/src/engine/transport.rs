use super::transport_tor::{shared_tor_client, tor_connect_retry_delay, tor_connect_timeout};
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::TcpStream;

use super::{StrictRemoteTarget, remote_models::StrictRemoteTransportKind};
use crate::endpoint_host_port;
use crate::endpoint_socket_addr;
use crate::remote_endpoint::test_tor_onion_dial_addr;
use crate::socket_addr_from_host_port;

// Tor transport uses an embedded arti_client::TorClient via `shared_tor_client`.
pub(crate) trait RemoteIo: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
impl<T> RemoteIo for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + ?Sized {}
pub(crate) type RemoteIoStream = Box<dyn RemoteIo + Unpin + Send>;

pub(crate) fn socket_addr(target: &StrictRemoteTarget) -> Result<String> {
    endpoint_socket_addr(&target.endpoint)
}

pub(crate) async fn connect(target: &StrictRemoteTarget) -> Result<RemoteIoStream> {
    match target.transport_kind {
        StrictRemoteTransportKind::Direct => connect_direct(target).await,
        StrictRemoteTransportKind::Tor => connect_tor(target).await,
    }
}

pub(crate) fn preflight_timeout(target: &StrictRemoteTarget) -> Duration {
    match target.transport_kind {
        StrictRemoteTransportKind::Direct => Duration::from_secs(1),
        StrictRemoteTransportKind::Tor => tor_connect_timeout(),
    }
}

pub(crate) fn phase_timeout(target: &StrictRemoteTarget, requested: Duration) -> Duration {
    requested.max(min_phase_timeout(target.transport_kind))
}

async fn connect_direct(target: &StrictRemoteTarget) -> Result<RemoteIoStream> {
    let socket_addr = socket_addr(target)?;
    let stream = TcpStream::connect(&socket_addr).await?;
    let stream: RemoteIoStream = Box::new(stream);
    Ok(stream)
}

async fn connect_tor(target: &StrictRemoteTarget) -> Result<RemoteIoStream> {
    let (host, port) = endpoint_host_port(&target.endpoint)?;
    if !host.ends_with(".onion") {
        let socket_addr = socket_addr_from_host_port(&host, port);
        let stream = TcpStream::connect(&socket_addr).await?;
        let stream: RemoteIoStream = Box::new(stream);
        return Ok(stream);
    }

    if let Some(test_dial_addr) = test_tor_onion_dial_addr() {
        loop {
            match TcpStream::connect(&test_dial_addr).await.with_context(|| {
                format!(
                    "infra error: remote node {} unavailable at {}",
                    target.node_id, target.endpoint
                )
            }) {
                Ok(stream) => return Ok(Box::new(stream)),
                Err(_) => tokio::time::sleep(tor_connect_retry_delay()).await,
            }
        }
    }

    let tor_client = shared_tor_client(target).await?;
    loop {
        match tor_client
            .connect((host.as_str(), port))
            .await
            .with_context(|| {
                format!(
                    "infra error: remote node {} unavailable at {}",
                    target.node_id, target.endpoint
                )
            }) {
            Ok(stream) => return Ok(Box::new(stream)),
            Err(_) => tokio::time::sleep(tor_connect_retry_delay()).await,
        }
    }
}

fn min_phase_timeout(transport_kind: StrictRemoteTransportKind) -> Duration {
    match transport_kind {
        StrictRemoteTransportKind::Direct => Duration::ZERO,
        StrictRemoteTransportKind::Tor => tor_connect_timeout(),
    }
}
