use std::collections::BTreeMap;

use tak_core::v2::{ContainerSource, EnvironmentValue, Step, TaskRuntime};
use tak_proto::worker_v2::{DispatchAttemptRequest, encode_dispatch_request, payload_digest};

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    remote_container::configure_fake_docker_env,
    v2_worker_cache::ensure,
    v2_worker_capacity::wait_terminal,
    v2_worker_execution::{output_archive, output_dispatch},
    v2_worker_http::{post, status},
    worker_http::{RunningServer, start_server_with_runtime},
};

#[path = "v2_worker_container_path_behavior/configured.rs"]
mod configured;
#[path = "v2_worker_container_path_behavior/path.rs"]
mod path;

async fn dispatch(server: &RunningServer, request: DispatchAttemptRequest) {
    ensure(
        server,
        &request.payload.workspace.descriptor,
        &output_archive(),
    )
    .await;
    let response = post(
        server,
        "/v2/attempts/dispatch",
        Some("secret"),
        &["v2"],
        &encode_dispatch_request(&request).unwrap(),
    )
    .await;
    assert_eq!(status(&response), 202);
    wait_terminal(server, &request).await;
}

fn request(id: &str, passed_path: Option<&str>, step_path: Option<&str>) -> DispatchAttemptRequest {
    let mut request = output_dispatch();
    request.identity.run_id = format!("run-{id}");
    request.identity.job_id = format!("job-{id}");
    request.identity.fencing_token = format!("fence-{id}");
    let task = &mut request.payload.tasks[0];
    task.job_id = request.identity.job_id.clone();
    task.runtime = Some(TaskRuntime::container(ContainerSource::Image {
        image: "alpine:3.20".into(),
    }));
    task.steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "true".into()],
        cwd: None,
        env: step_path
            .map(|path| BTreeMap::from([("PATH".into(), path.into())]))
            .unwrap_or_default(),
    }];
    task.outputs.clear();
    if let Some(path) = passed_path {
        task.pass_env_names = vec!["PATH".into()];
        request.payload.environment_values = vec![EnvironmentValue::new("PATH", path).unwrap()];
    }
    request.payload_digest = payload_digest(&request.payload).unwrap();
    request
}
