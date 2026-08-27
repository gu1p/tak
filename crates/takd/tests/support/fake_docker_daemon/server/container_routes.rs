use super::*;

mod status_filter;

use status_filter::requested_statuses;

pub(super) async fn write_container_inspect_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    path: &str,
) -> io::Result<()> {
    let container_id = path
        .split_once("/containers/")
        .and_then(|(_, tail)| tail.split('/').next())
        .unwrap_or_default();
    if let Some(container_state) = state.container_state(container_id) {
        let oom_killed = state.oom_killed;
        let body = serde_json::json!({
            "Id": container_id,
            "State": {
                "Status": container_state,
                "Running": container_state == "running" || container_state == "paused",
                "Paused": container_state == "paused",
                "OOMKilled": oom_killed
            },
        });
        write_response(
            stream,
            "200 OK",
            "application/json",
            body.to_string().as_bytes(),
        )
        .await
    } else {
        write_response(
            stream,
            "404 Not Found",
            "application/json",
            br#"{"message":"No such container"}"#,
        )
        .await
    }
}

pub(super) async fn write_container_list_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    request: &FakeDockerRequest,
) -> io::Result<()> {
    let records = state.container_summaries();
    let statuses = requested_statuses(request);
    if statuses.is_empty() {
        state.apply_post_list_transitions();
    }
    let containers = records
        .into_iter()
        .filter(|record| statuses.is_empty() || statuses.contains(&record.state))
        .map(|record| {
            serde_json::json!({
                "Id": record.container_id,
                "Names": [format!("/{}", record.container_id)],
                "Image": record.image.unwrap_or_default(),
                "Command": record.cmd.join(" "),
                "Labels": record.labels,
                "State": record.state,
                "Status": "Up",
            })
        })
        .collect::<Vec<_>>();
    write_response(
        stream,
        "200 OK",
        "application/json",
        serde_json::to_string(&containers)
            .expect("serialize fake container list")
            .as_bytes(),
    )
    .await
}

pub(super) async fn write_unpause_failure_response(
    stream: &mut UnixStream,
    state: &FakeDockerDaemonState,
    path: &str,
) -> io::Result<()> {
    let container_id = path
        .split_once("/containers/")
        .and_then(|(_, tail)| tail.split('/').next())
        .unwrap_or_default();
    state.record_container_unpause_attempt(container_id);
    write_response(
        stream,
        "500 Internal Server Error",
        "application/json",
        br#"{"message":"injected unpause failure"}"#,
    )
    .await
}
