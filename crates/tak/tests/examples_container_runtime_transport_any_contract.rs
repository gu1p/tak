mod support;

use anyhow::Result;

use support::examples_catalog::{CheckFileContains, ExampleEntry};
use support::examples_run::run_example;

fn example_entry(remote_fixture: &str, transport: &str, remote_node: &str) -> ExampleEntry {
    ExampleEntry {
        name: "large/29_remote_any_transport_container_log_storm".into(),
        run_target: "//apps/logstorm:observe_container_log_storm".into(),
        explain_target: "//apps/logstorm:observe_container_log_storm".into(),
        expect_success: true,
        requires_daemon: false,
        remote_fixture: Some(remote_fixture.into()),
        simulate_container_runtime: true,
        expect_stdout_contains: vec![
            "placement=remote".into(),
            format!("transport={transport}"),
            "runtime=containerized".into(),
            "runtime_engine=docker".into(),
            format!("remote_node={remote_node}"),
            "log-storm-stdout-001".into(),
        ],
        expect_stderr_contains: Vec::new(),
        check_files: vec![
            "out/local-input.txt".into(),
            "out/container-log-storm-summary.txt".into(),
            "out/container-log-storm-verified.txt".into(),
            "out/container-log-storm-report.txt".into(),
        ],
        check_file_contains: vec![
            CheckFileContains {
                path: "out/container-log-storm-summary.txt".into(),
                contains: "stderr_lines=60".into(),
            },
            CheckFileContains {
                path: "out/container-log-storm-verified.txt".into(),
                contains: "container-log-storm-verified".into(),
            },
        ],
    }
}

#[test]
fn container_log_storm_example_runs_with_simulated_direct_and_tor_fixtures() -> Result<()> {
    let direct_temp = tempfile::tempdir()?;
    let direct_entry = example_entry("direct_http", "direct", "remote-container-log-storm-direct");
    run_example(&direct_entry, direct_temp.path())?;

    let tor_temp = tempfile::tempdir()?;
    let tor_entry = example_entry("tor_onion_http", "tor", "remote-container-log-storm-tor");
    run_example(&tor_entry, tor_temp.path())
}
