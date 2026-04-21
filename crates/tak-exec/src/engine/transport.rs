use super::transport_tor::{shared_tor_client, tor_connect_retry_delay, tor_connect_timeout};
use super::*;
use crate::endpoint_host_port;
use crate::endpoint_socket_addr;
use crate::socket_addr_from_host_port;

trait RemoteTransportAdapter {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn socket_addr(&self, endpoint: &str) -> Result<String>;
    fn preflight_timeout(&self) -> Duration {
        Duration::from_secs(1)
    }
    fn min_phase_timeout(&self) -> Duration {
        Duration::ZERO
    }
    fn connect_stream<'a>(
        &'a self,
        target: &'a StrictRemoteTarget,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteIoStream>> + Send + 'a>>;
}

struct DirectHttpsTransportAdapter;
struct TorTransportAdapter;
pub(crate) trait RemoteIo: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
impl<T> RemoteIo for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + ?Sized {}
pub(crate) type RemoteIoStream = Box<dyn RemoteIo + Unpin + Send>;

impl RemoteTransportAdapter for DirectHttpsTransportAdapter {
    fn name(&self) -> &'static str {
        "direct"
    }
    fn socket_addr(&self, endpoint: &str) -> Result<String> {
        endpoint_socket_addr(endpoint)
    }
    fn connect_stream<'a>(
        &'a self,
        target: &'a StrictRemoteTarget,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteIoStream>> + Send + 'a>> {
        Box::pin(async move {
            let socket_addr = self.socket_addr(&target.endpoint)?;
            let stream = TcpStream::connect(&socket_addr).await?;
            let stream: RemoteIoStream = Box::new(stream);
            Ok(stream)
        })
    }
}

impl RemoteTransportAdapter for TorTransportAdapter {
    fn name(&self) -> &'static str {
        "tor"
    }
    fn socket_addr(&self, endpoint: &str) -> Result<String> {
        endpoint_socket_addr(endpoint)
    }
    fn preflight_timeout(&self) -> Duration {
        tor_connect_timeout()
    }
    fn min_phase_timeout(&self) -> Duration {
        tor_connect_timeout()
    }
    fn connect_stream<'a>(
        &'a self,
        target: &'a StrictRemoteTarget,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteIoStream>> + Send + 'a>> {
        Box::pin(async move {
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
        })
    }
}

static DIRECT_HTTPS_TRANSPORT_ADAPTER: DirectHttpsTransportAdapter = DirectHttpsTransportAdapter;
static TOR_TRANSPORT_ADAPTER: TorTransportAdapter = TorTransportAdapter;

fn transport_adapter_for_kind(kind: RemoteTransportKind) -> &'static dyn RemoteTransportAdapter {
    match kind {
        RemoteTransportKind::Any => {
            panic!("strict remote targets must resolve to a concrete transport")
        }
        RemoteTransportKind::Direct => &DIRECT_HTTPS_TRANSPORT_ADAPTER,
        RemoteTransportKind::Tor => &TOR_TRANSPORT_ADAPTER,
    }
}

pub(crate) struct TransportFactory;

impl TransportFactory {
    fn adapter(kind: RemoteTransportKind) -> &'static dyn RemoteTransportAdapter {
        transport_adapter_for_kind(kind)
    }

    #[allow(dead_code)]
    fn transport_name(kind: RemoteTransportKind) -> &'static str {
        Self::adapter(kind).name()
    }

    pub(crate) fn socket_addr(target: &StrictRemoteTarget) -> Result<String> {
        Self::adapter(target.transport_kind).socket_addr(&target.endpoint)
    }

    pub(crate) fn connect(
        target: &StrictRemoteTarget,
    ) -> impl Future<Output = Result<RemoteIoStream>> + Send + '_ {
        Self::adapter(target.transport_kind).connect_stream(target)
    }

    pub(crate) fn preflight_timeout(target: &StrictRemoteTarget) -> Duration {
        Self::adapter(target.transport_kind).preflight_timeout()
    }

    pub(crate) fn phase_timeout(target: &StrictRemoteTarget, requested: Duration) -> Duration {
        requested.max(Self::adapter(target.transport_kind).min_phase_timeout())
    }
}
