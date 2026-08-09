use super::*;

impl TorBroker {
    pub fn new() -> Self {
        match test_tor_onion_dial_addr() {
            Some(dial_addr) => Self::for_direct_dial(dial_addr),
            None => Self::with_options(None, None, false),
        }
    }

    /// Creates a broker that sends onion requests through a fixed direct dial address.
    ///
    /// ```rust
    /// let broker = takd::TorBroker::for_direct_dial("127.0.0.1:43123");
    /// # let _ = broker;
    /// ```
    pub fn for_direct_dial(dial_addr: impl AsRef<str>) -> Self {
        Self::with_options(Some(dial_addr.as_ref().to_string()), None, false)
    }

    pub fn for_state_root(state_root: PathBuf) -> Self {
        Self::with_options(test_tor_onion_dial_addr(), Some(state_root), false)
    }

    pub(in crate::daemon) fn with_options(
        test_dial_addr: Option<String>,
        state_root: Option<PathBuf>,
        requires_shared_client: bool,
    ) -> Self {
        Self {
            inner: Arc::new(TorBrokerInner {
                client: tokio::sync::OnceCell::const_new(),
                http2_sessions: tokio::sync::Mutex::new(HashMap::new()),
                http2_dials: Mutex::new(HashSet::new()),
                remote_protocols: tokio::sync::Mutex::new(HashMap::new()),
                test_dial_addr,
                state_root,
                shared_tor_client: Mutex::new(None),
                requires_shared_client,
            }),
        }
    }
}
