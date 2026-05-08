use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::direct_remote_runtime::{client_env, start_direct_agent};
use crate::support::{
    self, RemoteRecord, run_tak_expect_failure, run_tak_output, write_remote_inventory,
};

#[test]
fn remote_list_prints_generated_alias_for_node_selection() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    write_remote_inventory(&config_root, &[remote_record()])?;

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());
    let output = run_tak_output(temp.path(), &["remote", "list"], &env)?;
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("alias="), "stdout:\n{stdout}");
    assert!(stdout.contains("builder-node-123456"), "stdout:\n{stdout}");
    Ok(())
}

#[test]
fn docker_run_node_selector_accepts_generated_alias() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    write_remote_inventory(&config_root, &[remote_record()])?;

    let alias = support::remote_inventory::stable_remote_alias_for_test("builder-node-123456");
    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &[
            "--node",
            alias.as_str(),
            "--arch",
            "arm64",
            "--os",
            "linux",
            "docker",
            "run",
            "alpine:3.20",
            "true",
        ],
        &env,
    )?;

    assert!(!stderr.contains("no configured remote agents match tak docker run"));
    Ok(())
}

#[test]
fn docker_run_executes_image_command_on_remote_by_default() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let _agent = start_direct_agent(temp.path(), temp.path(), "docker-run-remote");

    let output = run_tak_output(
        temp.path(),
        &[
            "docker",
            "run",
            "alpine:3.20",
            "sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        &client_env(temp.path()),
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "image");
    Ok(())
}

fn remote_record() -> RemoteRecord {
    RemoteRecord {
        node_id: "builder-node-123456".into(),
        display_name: "builder-a".into(),
        base_url: "http://127.0.0.1:12345".into(),
        bearer_token: "secret".into(),
        pools: vec!["default".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into(), "arch:arm64".into(), "os:linux".into()],
        transport: "direct".into(),
        enabled: true,
    }
}
