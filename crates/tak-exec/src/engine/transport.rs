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
const DEFAULT_TOR_CONNECT_TIMEOUT: Duration = Duration::from_secs(120);
const DEFAULT_TOR_CONNECT_RETRY_DELAY: Duration = Duration::from_secs(1);

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
                let socket_addr = format!("{host}:{port}");
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

fn tor_connect_timeout() -> Duration {
    env_duration_ms(
        "TAK_TEST_TOR_PROBE_TIMEOUT_MS",
        "TAK_TOR_PROBE_TIMEOUT_MS",
        DEFAULT_TOR_CONNECT_TIMEOUT,
    )
}

fn tor_connect_retry_delay() -> Duration {
    env_duration_ms(
        "TAK_TEST_TOR_PROBE_BACKOFF_MS",
        "TAK_TOR_PROBE_BACKOFF_MS",
        DEFAULT_TOR_CONNECT_RETRY_DELAY,
    )
}

fn env_duration_ms(test_name: &str, live_name: &str, default: Duration) -> Duration {
    std::env::var(test_name)
        .ok()
        .or_else(|| std::env::var(live_name).ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_millis)
        .unwrap_or(default)
}

async fn shared_tor_client(
    target: &StrictRemoteTarget,
) -> Result<TorClient<tor_rtcompat::PreferredRuntime>> {
    static CELL: std::sync::OnceLock<
        tokio::sync::OnceCell<TorClient<tor_rtcompat::PreferredRuntime>>,
    > = std::sync::OnceLock::new();
    let client = CELL
        .get_or_init(tokio::sync::OnceCell::const_new)
        .get_or_try_init(|| async move {
            let deadline = tokio::time::Instant::now() + tor_connect_timeout();

            loop {
                match TorClient::create_bootstrapped(TorClientConfig::default()).await {
                    Ok(client) => return Ok(client),
                    Err(error) if tokio::time::Instant::now() >= deadline => {
                        return Err(error).with_context(|| {
                            format!(
                                "infra error: remote node {} unavailable at {}",
                                target.node_id, target.endpoint
                            )
                        });
                    }
                    Err(_) => {}
                }

                tokio::time::sleep(tor_connect_retry_delay()).await;
            }
        })
        .await?;
    Ok(client.clone())
}
