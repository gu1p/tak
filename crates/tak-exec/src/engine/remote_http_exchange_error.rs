#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteHttpExchangeErrorKind {
    Timeout,
    Connect,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteHttpExchangeError {
    pub(crate) kind: RemoteHttpExchangeErrorKind,
    pub(crate) message: String,
    retryable: bool,
    failed_node_id: Option<String>,
}

impl RemoteHttpExchangeError {
    pub(crate) fn timeout(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Timeout,
            message,
            retryable: true,
            failed_node_id: None,
        }
    }

    pub(crate) fn connect(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Connect,
            message,
            retryable: true,
            failed_node_id: None,
        }
    }

    pub(crate) fn other(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Other,
            message,
            retryable: false,
            failed_node_id: None,
        }
    }

    pub(crate) fn retryable_other(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Other,
            message,
            retryable: true,
            failed_node_id: None,
        }
    }

    pub(crate) fn is_retryable(&self) -> bool {
        self.retryable
    }

    pub(crate) fn with_failed_node_id(mut self, node_id: impl Into<String>) -> Self {
        self.failed_node_id = Some(node_id.into());
        self
    }

    pub(crate) fn failed_node_id(&self) -> Option<&str> {
        self.failed_node_id.as_deref()
    }
}

impl std::fmt::Display for RemoteHttpExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RemoteHttpExchangeError {}
