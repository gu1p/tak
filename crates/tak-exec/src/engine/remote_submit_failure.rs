#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteSubmitFailureKind {
    Auth,
    /// A submit that referenced a previously-uploaded workspace blob was rejected because the
    /// blob is no longer present on the node (it was reaped by the cleanup janitor). The
    /// caller should drop the cached reference, re-upload, and resubmit.
    MissingUpload,
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

    pub(crate) fn missing_upload(message: String) -> Self {
        Self {
            kind: RemoteSubmitFailureKind::MissingUpload,
            message,
            retryable: false,
        }
    }

    pub(crate) fn is_retryable(&self) -> bool {
        self.retryable
    }

    pub(crate) fn is_missing_upload(&self) -> bool {
        self.kind == RemoteSubmitFailureKind::MissingUpload
    }
}

impl std::fmt::Display for RemoteSubmitFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RemoteSubmitFailure {}
