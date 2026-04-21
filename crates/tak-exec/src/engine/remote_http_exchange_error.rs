#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteHttpExchangeErrorKind {
    Timeout,
    Connect,
    Other,
}

#[derive(Debug, Clone)]
struct RemoteHttpExchangeError {
    kind: RemoteHttpExchangeErrorKind,
    message: String,
}

impl RemoteHttpExchangeError {
    fn timeout(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Timeout,
            message,
        }
    }

    fn connect(message: String) -> Self {
        Self {
            kind: RemoteHttpExchangeErrorKind::Connect,
            message,
        }
    }

    fn other(message: String) -> Self {
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
