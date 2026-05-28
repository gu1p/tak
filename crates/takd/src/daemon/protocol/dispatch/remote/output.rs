use super::*;

pub(super) async fn get_output_range(
    request_id: String,
    task: crate::daemon::protocol::daemon_tasks::DaemonTaskHandle,
    payload: GetOutputRangeRequest,
    peers: &crate::daemon::peer_manager::PeerManager,
    broker: &TorBroker,
) -> Result<Response> {
    let path = output_range_path(&task.task_run_id, payload.attempt, &payload.path);
    let mut request = task_request(request_id, task.node_id, "GET", path);
    request.headers = range_header(payload.range);
    forward_task_http(request, peers, broker).await
}

fn output_range_path(task_run_id: &str, attempt: u32, output_path: &str) -> String {
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    query.append_pair("attempt", &attempt.to_string());
    query.append_pair("path", output_path);
    format!("/v1/tasks/{task_run_id}/outputs?{}", query.finish())
}

fn range_header(range: Option<String>) -> Vec<RemoteResponseHeader> {
    range
        .map(|value| {
            vec![RemoteResponseHeader {
                name: hyper::header::RANGE.as_str().to_string(),
                value,
            }]
        })
        .unwrap_or_default()
}
