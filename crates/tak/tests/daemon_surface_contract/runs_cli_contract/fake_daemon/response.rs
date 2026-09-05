use {super::Reply, serde_json::json};

#[path = "response/dashboard_attach.rs"]
mod dashboard_attach;
#[path = "response/empty_remote.rs"]
mod empty_remote;
#[path = "response/foreground_output.rs"]
mod foreground_output;
#[path = "response/management.rs"]
mod management;
#[path = "response/output.rs"]
mod output;
#[path = "response/preflight_conflict.rs"]
mod preflight_conflict;
#[path = "response/raw.rs"]
mod raw;
#[path = "response/recovery.rs"]
mod recovery;
#[path = "response/submission.rs"]
mod submission;
#[path = "response/terminal_output.rs"]
mod terminal_output;
use management::{failed_attach_response, management_response};
use output::{huge_output_response, symlink_chain_output_response, unsafe_output_response};
use submission::submission_response;

pub(super) fn response_bytes(
    reply: &Reply,
    request_id: &str,
    request: &serde_json::Value,
    request_number: usize,
) -> Option<Vec<u8>> {
    let value = match reply {
        Reply::Inactive(message) | Reply::SlowDripInactive(message, _, _) => json!({
            "protocol_version": 2,
            "type": "Error",
            "request_id": request_id,
            "message": message,
            "code": "protocol_v2_not_active",
            "retryable": false,
        }),
        Reply::Legacy(message) => json!({
            "type": "Error", "request_id": request_id, "message": message,
        }),
        Reply::Retryable(message) => json!({
            "protocol_version": 2,
            "type": "Error",
            "request_id": request_id,
            "message": message,
            "code": "protocol_v2_not_active",
            "retryable": true,
        }),
        Reply::Success => json!({
            "protocol_version": 2, "type": "RunList", "request_id": request_id, "runs": [],
        }),
        Reply::SubmissionFlow => submission_response(request_id, request, false, None),
        Reply::WorkspacePresentSubmissionFlow => recovery::present(request_id, request),
        Reply::ResumableSubmissionFlow(next_offset) => {
            recovery::resumable(request_id, request, *next_offset)
        }
        Reply::DelayedOutputSubmissionFlow(_) => foreground_output::response(request_id, request),
        Reply::RemoteSubmissionFlow => submission::remote_submission_response(request_id, request),
        Reply::EmptyRemoteSubmissionFlow => empty_remote::response(request_id, request),
        Reply::DelayedSubmissionFlow(_, _) => {
            submission_response(request_id, request, false, Some("cancelled"))
        }
        Reply::DelayedCancellationFlow(_, _, state) => {
            submission_response(request_id, request, false, Some(state))
        }
        Reply::AttachCancellationFlow(_) => {
            recovery::attach_cancellation(request_id, request, request_number)
        }
        Reply::FailedSubmissionFlow => submission_response(request_id, request, true, None),
        Reply::RetrySubmissionFlow if request_number == 0 => return None,
        Reply::RetrySubmissionFlow => submission_response(request_id, request, false, None),
        Reply::ManagementFlow => management_response(request_id, request),
        Reply::DashboardAttachFlow => dashboard_attach::response(request_id, request),
        Reply::ExpiredDashboardAttachFlow => {
            dashboard_attach::expired_response(request_id, request)
        }
        Reply::InteractiveDashboardFlow(_) | Reply::InteractiveDashboardCancellationFlow(_) => {
            dashboard_attach::interactive_response(request_id, request, request_number)
        }
        Reply::FailedAttachFlow => failed_attach_response(request_id, request),
        Reply::TerminalOutputFlow(state, _) => {
            terminal_output::response(request_id, request, state)
        }
        Reply::PreflightConflictFlow(_) => preflight_conflict::response(request_id, request),
        Reply::UnsafeOutputFlow => unsafe_output_response(request_id, request),
        Reply::SymlinkChainOutputFlow => symlink_chain_output_response(request_id, request),
        Reply::HugeOutputFlow => huge_output_response(request_id, request),
        Reply::Raw(bytes) | Reply::RawThenStall(bytes) => {
            return Some(raw::response(bytes, request_id, request));
        }
        Reply::Close => return None,
    };
    let mut bytes = serde_json::to_vec(&value).expect("encode fake response");
    bytes.push(b'\n');
    Some(bytes)
}
