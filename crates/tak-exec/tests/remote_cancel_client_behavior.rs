use std::fs;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunCancellation, RunOptions, run_tasks};

use crate::support::{
    EnvGuard, NonTerminalEventsServer, RemoteInventoryRecord, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[test]
fn cancelling_remote_run_posts_cancel_to_accepted_worker() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());

    let server = NonTerminalEventsServer::spawn();
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-non-terminal",
            &server.base_url,
            "secret",
            "direct",
        )],
    );
    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_cancel",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(async {
        let cancellation = RunCancellation::new();
        let run_cancellation = cancellation.clone();
        let labels = vec![label];
        let run = tokio::spawn(async move {
            run_tasks(
                &spec,
                &labels,
                &RunOptions {
                    cancellation: run_cancellation,
                    ..RunOptions::default()
                },
            )
            .await
        });

        wait_for_event_poll(&server).await;
        cancellation.cancel();
        let result = tokio::time::timeout(Duration::from_secs(3), run)
            .await
            .expect("run should stop after cancellation")
            .expect("join run task");
        let error = result.expect_err("cancelled run should fail");
        assert!(error.to_string().contains("cancelled"));
        assert_eq!(server.cancel_calls.load(Ordering::SeqCst), 1);
    });
}

async fn wait_for_event_poll(server: &NonTerminalEventsServer) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while server.events_calls.load(Ordering::SeqCst) == 0 {
        assert!(Instant::now() < deadline, "timed out waiting for events");
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
