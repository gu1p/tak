#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteSubmitFailureKind {
    Auth,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteSubmitFailure {
    pub(crate) kind: RemoteSubmitFailureKind,
    pub(crate) message: String,
    pub(crate) retryable: bool,
}

impl RemoteSubmitFailure {
    pub(crate) fn other(message: String) -> Self {
        Self {
            kind: RemoteSubmitFailureKind::Other,
            message,
            retryable: false,
        }
    }

    pub(crate) fn retryable_other(message: String) -> Self {
        Self {
            kind: RemoteSubmitFailureKind::Other,
            message,
            retryable: true,
        }
    }

    pub(crate) fn auth(message: String) -> Self {
        Self {
            kind: RemoteSubmitFailureKind::Auth,
            message,
            retryable: false,
        }
    }

    pub(crate) fn is_retryable(&self) -> bool {
        self.retryable
    }
}

impl std::fmt::Display for RemoteSubmitFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RemoteSubmitFailure {}
