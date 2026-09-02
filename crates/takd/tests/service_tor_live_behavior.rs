#![allow(clippy::await_holding_lock)]

use crate::support;

use std::fs;

use tak_proto::decode_tor_invite;

use support::env::env_lock;
use support::live_tor_cli::{LiveTorRoots, init_tor_agent, spawn_tor_agent, wait_for_token};
use support::live_tor_http::wait_for_onion_node_status;
use takd::agent::read_config;

#[tokio::test(flavor = "multi_thread")]
async fn serve_agent_real_tor_publishes_idle_remote_capacity() {
    let _env_lock = env_lock();
    fs::create_dir_all(".tmp").expect("create test temp root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let relative_temp = std::path::Path::new(".tmp")
        .join(temp.path().file_name().expect("temporary directory name"));
    let roots = LiveTorRoots::new(&relative_temp);

    init_tor_agent(&roots, "builder-tor-live");
    let bearer_token = read_config(&roots.config_root)
        .expect("read config")
        .bearer_token;
    let _child = spawn_tor_agent(&roots);

    let token = wait_for_token(&roots);
    let base_url = decode_tor_invite(&token).expect("decode tor invite");
    let status = wait_for_onion_node_status(&relative_temp, &base_url, &bearer_token).await;
    let fetched = status.node.expect("node status metadata");

    assert_eq!(fetched.node_id, "builder-tor-live");
    assert_eq!(fetched.base_url, base_url);
    assert_eq!(fetched.transport, "tor");
    assert!(status.active_jobs.is_empty(), "new agent should be idle");
    assert!(
        status.queued_jobs.is_empty(),
        "new agent queue should be empty"
    );
    assert!(
        status
            .cpu
            .and_then(|cpu| cpu.tak_admission_available_cores)
            .is_some_and(|cores| cores > 0.0),
        "ready idle Tor agent must advertise CPU admission capacity"
    );
    assert!(
        status
            .memory
            .and_then(|memory| memory.tak_admission_available_bytes)
            .is_some_and(|bytes| bytes > 0),
        "ready idle Tor agent must advertise memory admission capacity"
    );
    assert!(
        fs::read_to_string(roots.config_root.join("agent.toml"))
            .expect("read agent config")
            .contains(&base_url)
    );
}
