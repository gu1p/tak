use super::transport_tor::tor_connect_timeout;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::{TcpStream, UnixStream};

use super::{StrictRemoteTarget, remote_models::StrictRemoteTransportKind};
use crate::endpoint_host_port;
use crate::endpoint_socket_addr;
use crate::socket_addr_from_host_port;

// Tor onion transport goes through the local takd broker over TAKD_SOCKET.
pub(crate) trait RemoteIo: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
impl<T> RemoteIo for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + ?Sized {}
pub(crate) type RemoteIoStream = Box<dyn RemoteIo + Unpin + Send>;

pub(crate) struct RemoteConnection {
    pub(crate) stream: RemoteIoStream,
}

impl RemoteConnection {
    fn direct(stream: RemoteIoStream) -> Self {
        Self { stream }
    }

    fn broker(stream: RemoteIoStream) -> Self {
        Self { stream }
    }
}

pub(crate) fn socket_addr(target: &StrictRemoteTarget) -> Result<String> {
    endpoint_socket_addr(&target.endpoint)
}

pub(crate) async fn connect(target: &StrictRemoteTarget) -> Result<RemoteConnection> {
    if uses_tor_broker(target)? {
        return connect_tor_broker(target)
            .await
            .map(RemoteConnection::broker);
    }
    let stream = match target.transport_kind {
        StrictRemoteTransportKind::Direct => connect_direct(target).await,
        StrictRemoteTransportKind::Tor => connect_tor(target).await,
    }?;
    Ok(RemoteConnection::direct(stream))
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
    let socket_addr = socket_addr_from_host_port(&host, port);
    let stream = TcpStream::connect(&socket_addr).await?;
    Ok(Box::new(stream))
}

async fn connect_tor_broker(target: &StrictRemoteTarget) -> Result<RemoteIoStream> {
    let socket_path = broker_socket_path();
    let stream = UnixStream::connect(&socket_path).await.with_context(|| {
        if target.is_daemon_tor_placement() {
            return format!(
                "infra error: Tor remote execution requires local takd serve; local takd Tor broker unavailable at {} during remote placement",
                socket_path.display()
            );
        }
        format!(
            "infra error: Tor remote execution requires local takd serve; local takd Tor broker unavailable at {} while contacting remote node {}",
            socket_path.display(),
            target.node_id
        )
    })?;
    Ok(Box::new(stream))
}

fn min_phase_timeout(transport_kind: StrictRemoteTransportKind) -> Duration {
    match transport_kind {
        StrictRemoteTransportKind::Direct => Duration::ZERO,
        StrictRemoteTransportKind::Tor => tor_connect_timeout(),
    }
}

pub(crate) fn uses_tor_broker(target: &StrictRemoteTarget) -> Result<bool> {
    if target.transport_kind != StrictRemoteTransportKind::Tor {
        return Ok(false);
    }
    if target.daemon_task_handle.is_some() || target.is_daemon_tor_placement() {
        return Ok(true);
    }
    let (host, _) = endpoint_host_port(&target.endpoint)?;
    Ok(host.ends_with(".onion"))
}

pub(crate) fn broker_socket_path() -> PathBuf {
    std::env::var_os("TAKD_SOCKET")
        .map(PathBuf::from)
        .unwrap_or_else(default_broker_socket_path)
}

fn default_broker_socket_path() -> PathBuf {
    tak_core::runtime_paths::default_daemon_socket_path()
}
