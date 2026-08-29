use serde::Serialize;

use super::{PROTOCOL_VERSION, RequestDecodeError, RequestDecodeErrorCode};

const INACTIVE_MESSAGE: &str = "Protocol v2 run operations are not active in this takd build. Upgrade tak, takd, and workers together.";
const INVALID_VERSION_MESSAGE: &str = "protocol_version must appear exactly once as the integer 2.";
const UNSUPPORTED_VERSION_MESSAGE: &str =
    "This takd requires protocol v2. Upgrade tak, takd, and workers together.";
const INVALID_REQUEST_MESSAGE: &str = "Invalid protocol v2 request.";

/// A redacted protocol v2 error frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorResponse {
    protocol_version: u64,
    #[serde(rename = "type")]
    response_type: &'static str,
    request_id: Option<String>,
    message: &'static str,
    code: &'static str,
    retryable: bool,
}

impl ErrorResponse {
    /// Reports a recognized operation during the staged protocol gate.
    ///
    /// ```rust
    /// use tak_proto::local_daemon::v2::ErrorResponse;
    ///
    /// let value = serde_json::to_value(ErrorResponse::v2_not_active("r1".into()))?;
    /// assert_eq!(value["code"], "protocol_v2_not_active");
    /// # Ok::<(), serde_json::Error>(())
    /// ```
    pub fn v2_not_active(request_id: String) -> Self {
        Self::new(Some(request_id), INACTIVE_MESSAGE, "protocol_v2_not_active")
    }

    fn new(request_id: Option<String>, message: &'static str, code: &'static str) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            response_type: "Error",
            request_id,
            message,
            code,
            retryable: false,
        }
    }
}

impl From<RequestDecodeError> for ErrorResponse {
    fn from(error: RequestDecodeError) -> Self {
        let (message, code) = match error.code {
            RequestDecodeErrorCode::VersionInvalid => {
                (INVALID_VERSION_MESSAGE, "protocol_version_invalid")
            }
            RequestDecodeErrorCode::VersionUnsupported => {
                (UNSUPPORTED_VERSION_MESSAGE, "protocol_version_unsupported")
            }
            RequestDecodeErrorCode::RequestInvalid => {
                (INVALID_REQUEST_MESSAGE, "protocol_request_invalid")
            }
        };
        Self::new(error.request_id, message, code)
    }
}
