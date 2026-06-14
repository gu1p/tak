//! Classifies the HTTP status of a result/events fetch into a retry decision.

/// Classification of an HTTP status returned by a result/events fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FetchOutcome {
    /// 200 — a usable response body.
    Ok,
    /// Transient by HTTP semantics (5xx, 408, 429) — safe to retry/resume.
    Retryable,
    /// 404 — result not (yet) present.
    NotFound,
    /// Any other non-200 (ordinary 4xx, 3xx) — fail fast, retrying won't help.
    Terminal,
}

/// Maps an HTTP status to a retry decision. Status codes already encode
/// retryability, so the client classifies here rather than relying on a server
/// flag (mirrors the existing `broker_error_response` status classification).
///
/// ```no_run
/// # // Reason: This private classifier is exercised through result-fetch tests.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn classify_fetch_status(status: u16) -> FetchOutcome {
    match status {
        200 => FetchOutcome::Ok,
        404 => FetchOutcome::NotFound,
        408 | 429 | 500..=599 => FetchOutcome::Retryable,
        _ => FetchOutcome::Terminal,
    }
}

#[path = "status_tests.rs"]
mod status_tests;
