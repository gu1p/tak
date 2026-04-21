#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteSubmitFailureKind {
    Auth,
    Other,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteSubmitFailure {
    pub(crate) kind: RemoteSubmitFailureKind,
    pub(crate) message: String,
}

impl std::fmt::Display for RemoteSubmitFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RemoteSubmitFailure {}
