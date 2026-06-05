#[derive(Debug, Clone)]
pub(in crate::engine::protocol_result_http::request::daemon) enum DaemonLocalError {
    Response {
        message: String,
        code: Option<String>,
        retryable: Option<bool>,
    },
    Connect {
        message: String,
    },
    RetryableClient {
        message: String,
    },
}

impl DaemonLocalError {
    pub(in crate::engine::protocol_result_http::request::daemon) fn response(
        message: String,
        code: Option<String>,
        retryable: Option<bool>,
    ) -> Self {
        Self::Response {
            message,
            code,
            retryable,
        }
    }

    pub(in crate::engine::protocol_result_http::request::daemon) fn connect(
        message: String,
    ) -> Self {
        Self::Connect { message }
    }

    pub(in crate::engine::protocol_result_http::request::daemon) fn retryable_client(
        message: String,
    ) -> Self {
        Self::RetryableClient { message }
    }

    pub(in crate::engine::protocol_result_http::request::daemon) fn message(&self) -> &str {
        match self {
            Self::Response { message, .. }
            | Self::Connect { message }
            | Self::RetryableClient { message } => message,
        }
    }
}

impl std::fmt::Display for DaemonLocalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for DaemonLocalError {}
