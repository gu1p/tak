use std::collections::BTreeMap;

use tak_core::v2::{Affinity, JobEdge, RunSubmission, Session, SessionReuse, Step};
use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::{protocol_server::spawn_protocol_server, v2_run};

#[path = "v2_local_shared_workspace_behavior/context.rs"]
mod context;

#[tokio::test]
async fn dependent_local_attempts_execute_in_the_same_shared_workspace() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db).unwrap();
    let request = shared_run();
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    wait_for(|| {
        store
            .summary(&run_id)
            .unwrap()
            .is_some_and(|run| run.state.is_terminal())
    })
    .await;

    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Succeeded
    );
    assert!(store.events_after(&run_id, 0).unwrap().iter().any(|event| {
        event.kind == RunEventKind::Stdout
            && event.chunk_base64.as_deref() == Some("c2hhcmVkLWludGVncmF0aW9uCg==")
    }));
    server.abort();
}

fn shared_run() -> RunSubmission {
    let mut request = v2_run::submission("local-shared", "secret");
    let hard = Affinity::require_same_node("build").unwrap();
    let session = Session::new(
        "build",
        SessionReuse::shared_workspace(1).unwrap(),
        Some(hard.clone()),
    )
    .unwrap();
    request.run.tasks[0].task_id = "//:producer".into();
    request.run.tasks[0].steps = vec![shell("mkdir -p .shared; printf producer > .shared/value")];
    request.run.tasks[0].affinity = Some(hard.clone());
    request.run.jobs[0].task_ids = vec!["//:producer".into()];
    request.run.jobs[0].affinity = Some(hard.clone());
    request.run.jobs[0].session = Some(session.clone());
    let mut consumer = request.run.tasks[0].clone();
    consumer.task_id = "//:consumer".into();
    consumer.job_id = "job-1".into();
    consumer.dependencies = vec!["//:producer".into()];
    consumer.steps = vec![shell(
        "test \"$(cat .shared/value)\" = producer && printf 'shared-integration\\n'",
    )];
    let mut consumer_job = request.run.jobs[0].clone();
    consumer_job.job_id = "job-1".into();
    consumer_job.task_ids = vec!["//:consumer".into()];
    request.run.tasks.push(consumer);
    request.run.jobs.push(consumer_job);
    request.run.targets = vec!["//:consumer".into()];
    request.run.job_edges = vec![JobEdge {
        dependency_job_id: "job-0".into(),
        dependent_job_id: "job-1".into(),
    }];
    RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap()
}

fn shell(script: &str) -> Step {
    Step::Cmd {
        argv: vec!["/bin/sh".into(), "-c".into(), script.into()],
        cwd: None,
        env: BTreeMap::new(),
    }
}

async fn wait_for(predicate: impl Fn() -> bool) {
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        while !predicate() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
