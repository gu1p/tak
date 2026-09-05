use std::time::Duration;

use serde_json::Value;

use super::super::Reply;

pub(super) fn response_delay(
    reply: &Reply,
    request: &Value,
    request_number: usize,
) -> Option<Duration> {
    let operation = request["operation"]["type"].as_str();
    match reply {
        Reply::DelayedOutputSubmissionFlow(delay)
            if matches!(operation, Some("GetOutputManifest" | "GetOutputChunk")) =>
        {
            Some(*delay)
        }
        Reply::PreflightConflictFlow(delay)
            if matches!(operation, Some("GetOutputManifest" | "GetOutputChunk")) =>
        {
            Some(*delay)
        }
        Reply::DelayedSubmissionFlow(pending, delay) if operation == Some(pending) => Some(*delay),
        Reply::DelayedCancellationFlow(pending, delay, _)
            if operation == Some(pending) || operation == Some("CancelRun") =>
        {
            Some(*delay)
        }
        Reply::AttachCancellationFlow(_) if request_number == 0 => Some(Duration::from_secs(30)),
        Reply::AttachCancellationFlow(delay) if operation == Some("CancelRun") => Some(*delay),
        Reply::InteractiveDashboardFlow(delay)
            if operation == Some("AttachRun") && request_number == 1 =>
        {
            Some(*delay)
        }
        Reply::InteractiveDashboardCancellationFlow(_)
            if operation == Some("AttachRun") && request_number == 1 =>
        {
            Some(Duration::from_secs(30))
        }
        Reply::InteractiveDashboardCancellationFlow(delay) if operation == Some("CancelRun") => {
            Some(*delay)
        }
        Reply::TerminalOutputFlow(_, true)
            if operation == Some("AttachRun") && request_number == 3 =>
        {
            Some(Duration::from_secs(30))
        }
        _ => None,
    }
}
