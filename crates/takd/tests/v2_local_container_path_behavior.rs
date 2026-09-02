use std::collections::BTreeMap;
use std::time::Duration;

use tak_core::v2::{ContainerSource, EnvironmentValue, RunSubmission, Step, TaskRuntime};
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

use crate::support::{
    env::{EnvGuard, env_lock},
    fake_docker_daemon::{FakeDockerConfig, FakeDockerDaemon},
    protocol_server::spawn_protocol_server,
    remote_container::configure_fake_docker_env,
    v2_run,
};

#[path = "v2_local_container_path_behavior/configured.rs"]
mod configured;
#[path = "v2_local_container_path_behavior/path.rs"]
mod path;

fn container_run(key: &str, path: Option<&str>) -> RunSubmission {
    let mut request = v2_run::submission(key, "secret");
    let task = &mut request.run.tasks[0];
    task.runtime = Some(TaskRuntime::container(ContainerSource::Image {
        image: "alpine:3.20".into(),
    }));
    task.steps = vec![Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), "true".into()],
        cwd: None,
        env: BTreeMap::new(),
    }];
    task.pass_env_names = path.map(|_| vec!["PATH".into()]).unwrap_or_default();
    request.run.jobs[0].pass_env_names = task.pass_env_names.clone();
    let values = path
        .map(|value| vec![EnvironmentValue::new("PATH", value).unwrap()])
        .unwrap_or_default();
    RunSubmission::new(key, request.run, values).unwrap()
}

async fn commit_and_wait(store: &RunStore, request: RunSubmission) {
    let run_id = v2_run::scheduler::commit(store, &request, "alice");
    assert!(
        wait_for(|| {
            store
                .summary(&run_id)
                .unwrap()
                .is_some_and(|run| run.state.is_terminal())
        })
        .await,
        "run did not finish: {:?}; events: {:?}",
        store.summary(&run_id),
        store.events_after(&run_id, 0)
    );
    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Succeeded
    );
}

async fn wait_for(predicate: impl Fn() -> bool) -> bool {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .is_ok()
}
