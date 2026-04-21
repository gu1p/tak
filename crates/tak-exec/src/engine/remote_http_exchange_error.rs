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
}

impl RemoteHttpExchangeError {
    pub(crate) fn timeout(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Timeout,
            message,
        }
    }

    pub(crate) fn connect(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Connect,
            message,
        }
    }

    pub(crate) fn other(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Other,
            message,
        }
    }
}

impl std::fmt::Display for RemoteHttpExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RemoteHttpExchangeError {}
