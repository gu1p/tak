use crate::support;

use std::collections::BTreeMap;

use tak_loader::{LoadOptions, load_workspace};

use support::container_runtime::simulated_container_runtime_env;
use support::example_workspace::{assert_file_contains, stage_example_workspace};
use support::live_tor::{LiveTorRoots, init_tor_agent, spawn_tor_agent_with_env, wait_for_token};
use support::live_tor_remote::{
    add_remote_with_env, assert_remote_list, assert_remote_status_ok_with_env,
};
use support::local_daemon::LocalDaemonGuard;
use support::tor_smoke::{assert_success_with_log, tak_command, takd_bin};

#[test]
fn tor_smoke_runs_example_26_through_local_broker_and_roundtrips_artifacts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let roots = LiveTorRoots::new(temp.path());
    stage_example_workspace("large/26_remote_tor_artifact_roundtrip", &workspace_root);
    let spec = load_workspace(&workspace_root, &LoadOptions::default()).expect("load workspace");
    let broker_socket = temp.path().join("takd-broker.sock");
    let bind_addr = reserve_local_bind_addr();

    let takd = takd_bin();
    init_tor_agent(&takd, &roots, "remote-tor-artifacts");
    let mut serve_env = simulated_container_runtime_env(temp.path())
        .into_iter()
        .collect::<Vec<_>>();
    serve_env.push(("TAKD_TEST_TOR_HS_BIND_ADDR".into(), bind_addr.clone()));
    let mut client_env =
        BTreeMap::from([("TAK_TEST_TOR_ONION_DIAL_ADDR".into(), bind_addr.clone())]);
    let _child = spawn_tor_agent_with_env(&takd, &roots, &serve_env);
    let token = wait_for_token(&takd, &roots);
    add_remote_with_env(&workspace_root, &roots, &token, &client_env);
    let _broker = LocalDaemonGuard::spawn_with_tor_inventory(
        &broker_socket,
        &spec,
        bind_addr,
        roots.client_config_root.join("tak/remotes.toml"),
    );
    client_env.insert(
        "TAKD_SOCKET".into(),
        broker_socket.to_string_lossy().into_owned(),
    );
    assert_remote_list(&workspace_root, &roots, "remote-tor-artifacts");
    assert_remote_status_ok_with_env(&workspace_root, &roots, "remote-tor-artifacts", &client_env);
    roots.prepare_poisoned_client_ambient_dirs();

    let run = tak_command(&workspace_root, &roots.client_config_root)
        .env("TAKD_SOCKET", &broker_socket)
        .env("XDG_DATA_HOME", roots.poisoned_client_data_home())
        .env("XDG_CACHE_HOME", roots.poisoned_client_cache_home())
        .args(["run", "//:consume_remote_report"])
        .output()
        .expect("run tak over live tor");
    assert_success_with_log(
        &run,
        "tak run //:consume_remote_report",
        &roots.service_log_path(),
    );

    let run_stdout = String::from_utf8_lossy(&run.stdout);
    assert!(
        run_stdout.contains("placement=remote"),
        "missing remote placement summary:\n{run_stdout}"
    );
    assert!(
        run_stdout.contains("transport=tor"),
        "missing tor transport summary:\n{run_stdout}"
    );
    assert!(
        run_stdout.contains("remote_node=remote-tor-artifacts"),
        "missing tor node id in run summary:\n{run_stdout}"
    );
    assert_file_contains(
        &workspace_root.join("out/tor-remote-artifact.txt"),
        "tor-remote-artifact",
        "remote artifact",
    );
    assert_file_contains(
        &workspace_root.join("out/tor-remote.log"),
        "tor-transport-ok",
        "transport log",
    );
    assert_file_contains(
        &workspace_root.join("out/tor-roundtrip.txt"),
        "tor-roundtrip-local-ok",
        "roundtrip",
    );
}

fn reserve_local_bind_addr() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve bind addr");
    listener.local_addr().expect("local addr").to_string()
}
