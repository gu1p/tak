use std::collections::BTreeMap;
use std::time::Duration;

use tak_core::v2::Step;
use tak_proto::worker_v2::{WorkerTerminalOutcome, payload_digest};

use crate::support::{
    sqlite_gate,
    v2_worker_execution::{output_archive, output_dispatch},
    v2_worker_shared::{send, wait_terminal},
    worker_http::start_server,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transient_terminal_store_lock_does_not_replace_a_successful_child_result() {
    let server = start_server().await;
    let child_ready = server.state_root.join("terminal-child-ready");
    let release_child = server.state_root.join("release-terminal-child");
    let mut request = output_dispatch();
    request.payload.tasks[0].steps = vec![Step::Cmd {
        argv: vec![
            "/bin/sh".into(),
            "-c".into(),
            "printf ready > \"$CHILD_READY\"; while [ ! -e \"$RELEASE_CHILD\" ]; do sleep 0.01; done; exit 0".into(),
        ],
        cwd: None,
        env: BTreeMap::from([
            ("CHILD_READY".into(), child_ready.display().to_string()),
            ("RELEASE_CHILD".into(), release_child.display().to_string()),
        ]),
    }];
    request.payload.tasks[0].outputs.clear();
    request.payload_digest = payload_digest(&request.payload).unwrap();
    send(&server, &request, &output_archive()).await;
    tokio::time::timeout(Duration::from_secs(3), async {
        while !child_ready.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("child should reach its terminal gate");

    let gate = sqlite_gate::begin_immediate(&server.state_root.join("takd.sqlite"));
    std::fs::write(release_child, b"").unwrap();
    std::thread::sleep(Duration::from_millis(6_500));
    drop(gate);

    let observed = wait_terminal(&server, &request).await;
    let terminal = observed.terminal.unwrap();
    assert_eq!(terminal.outcome, WorkerTerminalOutcome::Succeeded);
    assert_eq!(terminal.exit_code, Some(0));
    assert!(observed.events.is_empty());
}
