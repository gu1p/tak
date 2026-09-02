use std::time::Duration;

use crate::support::{
    runtime_config,
    v2_worker_shared_retention::{dispatch, release_terminal_run, request, root, wait_for},
    worker_http::{restart, start_server_with_runtime},
};

#[tokio::test]
async fn periodic_gc_reclaims_only_explicitly_terminal_inactive_shared_runs() {
    let runtime = runtime_config::builder()
        .with_remote_cleanup_interval(Duration::from_millis(20))
        .build();
    let server = start_server_with_runtime(runtime).await;
    let active = request("active", "fence-active", "sleep 30");
    dispatch(&server, &active).await;
    let active_root = root(&server, &active);
    wait_for(&active_root, true).await;
    let terminal = request("terminal", "fence-terminal", "true");
    dispatch(&server, &terminal).await;
    let terminal_root = root(&server, &terminal);
    wait_for(&terminal_root, true).await;
    let unrelated = server.state_root.join("worker-v2-shared/unrelated");
    std::fs::create_dir_all(&unrelated).unwrap();
    std::fs::write(unrelated.join("identity.json"), b"not-json").unwrap();

    release_terminal_run(&server, &terminal).await;
    wait_for(&terminal_root, false).await;

    assert!(active_root.is_dir());
    assert!(unrelated.join("identity.json").is_file());
}

#[tokio::test]
async fn startup_gc_replays_persisted_terminal_release_without_touching_other_roots() {
    let slow = runtime_config::builder()
        .with_remote_cleanup_interval(Duration::from_secs(60))
        .build();
    let mut server = start_server_with_runtime(slow).await;
    let terminal = request("restart", "fence-restart", "true");
    dispatch(&server, &terminal).await;
    let terminal_root = root(&server, &terminal);
    wait_for(&terminal_root, true).await;
    release_terminal_run(&server, &terminal).await;
    assert!(terminal_root.is_dir());
    let unrelated = server.state_root.join("outside-shared-root/value");
    std::fs::create_dir_all(&unrelated).unwrap();

    let fast = runtime_config::builder()
        .with_remote_cleanup_interval(Duration::from_millis(20))
        .build();
    restart(&mut server, fast).await;
    wait_for(&terminal_root, false).await;

    assert!(unrelated.is_dir());
}
