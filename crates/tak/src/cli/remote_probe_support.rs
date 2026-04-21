use anyhow::Error;
use tokio::io::{AsyncRead, AsyncWrite};

pub(super) mod http;
pub(super) mod transport;

pub(super) trait RemoteIo: AsyncRead + AsyncWrite {}
impl<T> RemoteIo for T where T: AsyncRead + AsyncWrite + ?Sized {}
pub(super) type RemoteStream = Box<dyn RemoteIo + Unpin + Send>;

pub(super) enum ProbeAttemptError {
    Retryable(Error),
    Final(Error),
}

pub(super) struct AbortOnDrop<T> {
    handle: Option<tokio::task::JoinHandle<T>>,
}

impl<T> AbortOnDrop<T> {
    pub(super) fn new(handle: tokio::task::JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
}

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

impl ProbeAttemptError {
    pub(super) fn retryable(err: Error) -> Self {
        Self::Retryable(err)
    }

    pub(super) fn final_error(err: Error) -> Self {
        Self::Final(err)
    }

    pub(super) fn is_retryable(&self) -> bool {
        matches!(self, Self::Retryable(_))
    }

    pub(super) fn into_anyhow(self) -> Error {
        match self {
            Self::Retryable(err) | Self::Final(err) => err,
        }
    }
}

mod remote_probe_support_request_tests;
mod remote_probe_support_tests;
