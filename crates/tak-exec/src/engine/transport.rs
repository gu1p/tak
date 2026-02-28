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
trait RemoteIo: tokio::io::AsyncRead + tokio::io::AsyncWrite {}
impl<T> RemoteIo for T where T: tokio::io::AsyncRead + tokio::io::AsyncWrite + ?Sized {}
type RemoteIoStream = Box<dyn RemoteIo + Unpin + Send>;

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
        Duration::from_secs(30)
    }

    fn min_phase_timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn connect_stream<'a>(
        &'a self,
        target: &'a StrictRemoteTarget,
    ) -> Pin<Box<dyn Future<Output = Result<RemoteIoStream>> + Send + 'a>> {
        Box::pin(async move {
            let (host, port) = endpoint_host_port(&target.endpoint)?;
            if !host.ends_with(".onion") {
                let socket_addr = format!("{host}:{port}");
                let stream = TcpStream::connect(&socket_addr).await?;
                let stream: RemoteIoStream = Box::new(stream);
                return Ok(stream);
            }

            if let Some(test_dial_addr) = test_tor_onion_dial_addr() {
                let stream = TcpStream::connect(&test_dial_addr).await.with_context(|| {
                    format!(
                        "infra error: remote node {} unavailable at {}",
                        target.node_id, target.endpoint
                    )
                })?;
                let stream: RemoteIoStream = Box::new(stream);
                return Ok(stream);
            }

            let tor_client = TorClient::create_bootstrapped(TorClientConfig::default())
                .await
                .with_context(|| {
                    format!(
                        "infra error: remote node {} unavailable at {}",
                        target.node_id, target.endpoint
                    )
                })?;
            let stream = tor_client
                .connect((host.as_str(), port))
                .await
                .with_context(|| {
                    format!(
                        "infra error: remote node {} unavailable at {}",
                        target.node_id, target.endpoint
                    )
                })?;
            let stream: RemoteIoStream = Box::new(stream);
            Ok(stream)
        })
    }
}

static DIRECT_HTTPS_TRANSPORT_ADAPTER: DirectHttpsTransportAdapter = DirectHttpsTransportAdapter;
static TOR_TRANSPORT_ADAPTER: TorTransportAdapter = TorTransportAdapter;

struct TransportFactory;

impl TransportFactory {
    fn adapter(kind: RemoteTransportKind) -> &'static dyn RemoteTransportAdapter {
        transport_adapter_for_kind(kind)
    }

    #[allow(dead_code)]
    fn transport_name(kind: RemoteTransportKind) -> &'static str {
        Self::adapter(kind).name()
    }

    fn socket_addr(target: &StrictRemoteTarget) -> Result<String> {
        Self::adapter(target.transport_kind).socket_addr(&target.endpoint)
    }

    fn connect(
        target: &StrictRemoteTarget,
    ) -> impl Future<Output = Result<RemoteIoStream>> + Send + '_ {
        Self::adapter(target.transport_kind).connect_stream(target)
    }

    fn preflight_timeout(target: &StrictRemoteTarget) -> Duration {
        Self::adapter(target.transport_kind).preflight_timeout()
    }

    fn phase_timeout(target: &StrictRemoteTarget, requested: Duration) -> Duration {
        requested.max(Self::adapter(target.transport_kind).min_phase_timeout())
    }
}
