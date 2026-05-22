use std::collections::BTreeMap;
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use anyhow::Result;

#[path = "run_remote_shuffle/recording.rs"]
mod recording;

use crate::support::{RemoteRecord, run_tak_output, write_remote_inventory};
use recording::RecordingDockerRunNode;

#[test]
fn docker_run_default_remote_selection_uses_shuffle_load_awareness() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    let unknown = RecordingDockerRunNode::spawn("builder-unknown", false);
    let known = RecordingDockerRunNode::spawn("builder-known", true);
    write_remote_inventory(
        &config_root,
        &[
            remote_record_for(&unknown.node_id, &unknown.base_url),
            remote_record_for(&known.node_id, &known.base_url),
        ],
    )?;

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());
    let output = run_tak_output(temp.path(), &["docker", "run", "alpine:3.20", "true"], &env)?;

    assert_command_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "builder-known"
    );
    Ok(())
}

#[test]
fn docker_run_recording_node_survives_disconnected_probe() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    let node = RecordingDockerRunNode::spawn("builder-known", true);
    write_remote_inventory(
        &config_root,
        &[remote_record_for(&node.node_id, &node.base_url)],
    )?;

    TcpStream::connect(node_authority(&node.base_url))?;
    thread::sleep(Duration::from_millis(20));

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());
    let output = run_tak_output(temp.path(), &["docker", "run", "alpine:3.20", "true"], &env)?;

    assert_command_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "builder-known"
    );
    Ok(())
}

fn remote_record_for(node_id: &str, base_url: &str) -> RemoteRecord {
    RemoteRecord {
        node_id: node_id.into(),
        display_name: node_id.into(),
        base_url: base_url.into(),
        bearer_token: "secret".into(),
        pools: vec!["default".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into(), "arch:arm64".into(), "os:linux".into()],
        transport: "direct".into(),
        enabled: true,
    }
}

fn node_authority(base_url: &str) -> &str {
    base_url
        .strip_prefix("http://")
        .expect("recording node uses http")
}

fn assert_command_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "status: {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
