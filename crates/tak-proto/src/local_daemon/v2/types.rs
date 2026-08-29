/// The safe result of classifying one local-daemon frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeOutcome {
    /// No top-level v2 marker was observed; the staged legacy decoder may try it.
    LegacyCandidate,
    /// A complete, strict protocol v2 request.
    V2(Request),
}

/// A decoded protocol v2 request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub request_id: String,
    pub operation: Operation,
}

/// A daemon-owned run operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    ListRuns {},
    GetRun { run_id: String },
    AttachRun { run_id: String, after_event: u64 },
    CancelRun { run_id: String },
}

/// A fixed protocol-v2 request classification failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestDecodeError {
    pub code: RequestDecodeErrorCode,
    pub request_id: Option<String>,
}

/// Stable categories used to build redacted wire errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestDecodeErrorCode {
    VersionInvalid,
    VersionUnsupported,
    RequestInvalid,
}
