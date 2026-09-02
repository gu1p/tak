use std::collections::BTreeMap;
use std::time::Duration;

use tak_core::v2::{RunSubmission, Step};
use tak_proto::local_daemon::v2::{RunEventKind, RunLifecycleState};
use takd::RunStore;

use crate::support::{protocol_server::spawn_protocol_server, v2_run};

#[tokio::test]
async fn real_v2_server_drives_a_local_attempt_and_persists_its_output() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let db = temp.path().join("takd.sqlite");
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let server = spawn_protocol_server(db.clone(), socket.clone());
    wait_for(|| socket.exists()).await;
    let store = RunStore::with_db_path(db).unwrap();
    let mut request = v2_run::submission("server-local", "secret");
    request.run.tasks[0].steps = vec![Step::Cmd {
        argv: vec![
            "/bin/sh".into(),
            "-c".into(),
            "test \"$TOKEN\" = secret && printf 'server-local-output\\n'".into(),
        ],
        cwd: None,
        env: BTreeMap::new(),
    }];
    let request = RunSubmission::new(
        request.idempotency_key,
        request.run,
        request.environment_values,
    )
    .unwrap();
    let run_id = v2_run::scheduler::commit(&store, &request, "alice");
    let terminal = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if store.summary(&run_id).unwrap().unwrap().state.is_terminal() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await;
    assert!(
        terminal.is_ok(),
        "daemon did not execute committed local run: {:?} / {:?}",
        store.summary(&run_id).unwrap(),
        store.events_after(&run_id, 0).unwrap()
    );
    assert_eq!(
        store.summary(&run_id).unwrap().unwrap().state,
        RunLifecycleState::Succeeded
    );
    let events = store.events_after(&run_id, 0).unwrap();
    assert!(
        events
            .iter()
            .any(|event| event.kind == RunEventKind::Running)
    );
    assert!(events.iter().any(|event| {
        event.kind == RunEventKind::Stdout
            && event.chunk_base64.as_deref() == Some("c2VydmVyLWxvY2FsLW91dHB1dAo=")
    }));
    server.abort();
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
