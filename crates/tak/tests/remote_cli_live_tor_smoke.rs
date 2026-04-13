mod support;

use support::example_workspace::{assert_file_contains, stage_example_workspace};
use support::live_tor::{LiveTorRoots, init_tor_agent, spawn_tor_agent, wait_for_token};
use support::live_tor_remote::{add_remote, assert_remote_list, assert_remote_status_ok};
use support::tor_smoke::{assert_success_with_log, tak_command, takd_bin};

#[test]
fn live_tor_smoke_runs_example_26_over_real_onion_and_roundtrips_artifacts() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let roots = LiveTorRoots::new(temp.path());
    stage_example_workspace("large/26_remote_tor_artifact_roundtrip", &workspace_root);

    let takd = takd_bin();
    init_tor_agent(&takd, &roots, "remote-tor-artifacts");
    let _child = spawn_tor_agent(&takd, &roots);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);
    assert_remote_list(&workspace_root, &roots, "remote-tor-artifacts");
    assert_remote_status_ok(&workspace_root, &roots, "remote-tor-artifacts");
    roots.prepare_poisoned_client_ambient_dirs();

    let run = tak_command(&workspace_root, &roots.client_config_root)
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
