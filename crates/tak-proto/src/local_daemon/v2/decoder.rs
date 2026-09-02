use super::probe::{Decision, Probe};
use super::{DecodeOutcome, RequestDecodeError};

/// Classifies and strictly decodes one local-daemon request frame.
///
/// Versionless and mismatched requests fail closed with upgrade guidance.
///
/// ```rust
/// use tak_proto::local_daemon::v2::{DecodeOutcome, decode_request};
///
/// let raw = r#"{"protocol_version":2,"request_id":"list","operation":{"type":"ListRuns"}}"#;
/// assert!(matches!(decode_request(raw), Ok(DecodeOutcome::V2(_))));
/// ```
pub fn decode_request(raw: &str) -> Result<DecodeOutcome, RequestDecodeError> {
    if raw.len() > super::MAX_REQUEST_FRAME_BYTES {
        return Err(RequestDecodeError {
            code: super::RequestDecodeErrorCode::RequestInvalid,
            request_id: None,
        });
    }
    let probe = Probe::inspect(raw);
    match probe.decide() {
        Decision::Reject(code) => Err(RequestDecodeError {
            code,
            request_id: probe.request_id(),
        }),
        Decision::StrictV2 => super::wire::decode_strict(raw)
            .map(DecodeOutcome::V2)
            .ok_or_else(|| RequestDecodeError {
                code: super::RequestDecodeErrorCode::RequestInvalid,
                request_id: probe.request_id(),
            }),
    }
}
