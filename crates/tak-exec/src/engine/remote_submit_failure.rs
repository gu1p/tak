#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteSubmitFailureKind {
    Auth,
    Other,
}

#[derive(Debug, Clone)]
struct RemoteSubmitFailure {
    kind: RemoteSubmitFailureKind,
    message: String,
}

impl std::fmt::Display for RemoteSubmitFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for RemoteSubmitFailure {}
