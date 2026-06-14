//! Renders an events/result fetch failure for the event-stream loop into the
//! shared rich error string.

use crate::engine::remote_result_fetch::{RemoteFetchFailure, format_remote_fetch_failure};
use crate::engine::{RemoteHttpExchangeError, StrictRemoteTarget};

/// The unchanging identity of one event-stream attempt, shared by every failure
/// rendered while that attempt runs.
pub(super) struct EventStreamTarget<'a> {
    pub(super) target: &'a StrictRemoteTarget,
    pub(super) task_run_id: &'a str,
    pub(super) attempt: u32,
}

pub(super) fn event_stream_failure(
    stream: &EventStreamTarget<'_>,
    phase: &str,
    path: &str,
    status: Option<u16>,
    body: Option<&[u8]>,
    transport_error: Option<&RemoteHttpExchangeError>,
) -> String {
    format_remote_fetch_failure(&RemoteFetchFailure {
        target: stream.target,
        task_run_id: stream.task_run_id,
        attempt: stream.attempt,
        phase,
        path,
        status,
        body,
        transport_error,
    })
}
