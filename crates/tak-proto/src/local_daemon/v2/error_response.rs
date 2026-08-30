use serde::Serialize;

use super::{PROTOCOL_VERSION, RequestDecodeError, RequestDecodeErrorCode};

const INACTIVE_MESSAGE: &str = "Protocol v2 run operations are not active in this takd build. Upgrade tak, takd, and workers together.";
const INVALID_VERSION_MESSAGE: &str = "protocol_version must appear exactly once as the integer 2.";
const UNSUPPORTED_VERSION_MESSAGE: &str =
    "This takd requires protocol v2. Upgrade tak, takd, and workers together.";
const INVALID_REQUEST_MESSAGE: &str = "Invalid protocol v2 request.";
const IDEMPOTENCY_CONFLICT_MESSAGE: &str =
    "The idempotency key is already bound to a different run submission.";
const RUN_NOT_FOUND_MESSAGE: &str = "The requested run does not exist.";
const WORKSPACE_INVALID_MESSAGE: &str = "The workspace upload is invalid.";
const RUN_STATE_INVALID_MESSAGE: &str = "The requested operation is invalid for this run state.";
const INTERNAL_MESSAGE: &str = "The local daemon could not complete the request.";

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

    pub fn idempotency_conflict(request_id: String) -> Self {
        Self::new(
            Some(request_id),
            IDEMPOTENCY_CONFLICT_MESSAGE,
            "idempotency_conflict",
        )
    }

    pub fn run_not_found(request_id: String) -> Self {
        Self::new(Some(request_id), RUN_NOT_FOUND_MESSAGE, "run_not_found")
    }

    pub fn workspace_invalid(request_id: String) -> Self {
        Self::new(
            Some(request_id),
            WORKSPACE_INVALID_MESSAGE,
            "workspace_invalid",
        )
    }

    pub fn run_state_invalid(request_id: String) -> Self {
        Self::new(
            Some(request_id),
            RUN_STATE_INVALID_MESSAGE,
            "run_state_invalid",
        )
    }

    pub fn internal(request_id: String) -> Self {
        Self::new(Some(request_id), INTERNAL_MESSAGE, "internal")
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
