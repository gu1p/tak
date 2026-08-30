use std::collections::BTreeMap;
use std::time::Duration;

use base64::Engine;
use tak_core::v2::{RunSubmission, Step};
use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::{protocol_server::spawn_protocol_server, v2_run};

#[tokio::test]
async fn fused_local_job_attributes_each_output_chunk_to_its_task() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db).unwrap();
    let mut request = v2_run::submission("fused-output", "secret");
    let mut first = request.run.tasks[0].clone();
    first.task_id = "//:first".into();
    first.steps = vec![print("first-output")];
    let mut second = request.run.tasks[0].clone();
    second.task_id = "//:second".into();
    second.dependencies = vec![first.task_id.clone()];
    second.steps = vec![print("second-output")];
    request.run.targets = vec![second.task_id.clone()];
    request.run.tasks = vec![first, second];
    request.run.jobs[0].task_ids = vec!["//:first".into(), "//:second".into()];
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    wait_for(|| {
        store
            .summary(&run_id)
            .unwrap()
            .is_some_and(|run| run.state == RunLifecycleState::Succeeded)
    })
    .await;
    let mut output = BTreeMap::<String, Vec<u8>>::new();
    for event in store
        .events_after(&run_id, 0)
        .unwrap()
        .into_iter()
        .filter(|event| event.kind == RunEventKind::Stdout)
    {
        let [task_id] = event.task_ids.as_slice() else {
            panic!("output event must name exactly one fused task")
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(event.chunk_base64.unwrap())
            .unwrap();
        output.entry(task_id.clone()).or_default().extend(bytes);
    }
    assert_eq!(output["//:first"], b"first-output");
    assert_eq!(output["//:second"], b"second-output");
    server.abort();
}

fn print(value: &str) -> Step {
    Step::Cmd {
        argv: vec!["/usr/bin/printf".into(), value.into()],
        cwd: None,
        env: BTreeMap::new(),
    }
}

async fn wait_for(predicate: impl Fn() -> bool) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !predicate() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
