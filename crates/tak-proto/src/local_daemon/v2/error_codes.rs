use std::fmt::{Display, Formatter};

/// A daemon-owned error code accepted by the staged v2 client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonErrorCode {
    ProtocolV2NotActive,
    ProtocolVersionInvalid,
    ProtocolVersionUnsupported,
    ProtocolRequestInvalid,
    IdempotencyConflict,
    RunNotFound,
    WorkspaceInvalid,
    RunStateInvalid,
    Internal,
}

/// A fixed request-encoding failure that never contains authored values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestEncodeError {
    RequestIdInvalid,
    RunIdInvalid,
    PayloadInvalid,
    FrameTooLarge,
    EncodingFailed,
}

impl Display for RequestEncodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::RequestIdInvalid => "protocol v2 request id is invalid",
            Self::RunIdInvalid => "protocol v2 run id is invalid",
            Self::PayloadInvalid => "protocol v2 request payload is invalid",
            Self::FrameTooLarge => "protocol v2 request frame is too large",
            Self::EncodingFailed => "protocol v2 request encoding failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RequestEncodeError {}

/// A fixed response classification that never retains untrusted bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseDecodeError {
    ProtocolMismatch,
    FrameTooLarge,
}

impl Display for ResponseDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::ProtocolMismatch => "protocol v2 response mismatch",
            Self::FrameTooLarge => "protocol v2 response frame is too large",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ResponseDecodeError {}
