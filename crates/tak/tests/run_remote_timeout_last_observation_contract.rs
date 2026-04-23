use crate::support;

use std::collections::BTreeMap;
use std::io::Read;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use tak_exec::write_remote_observation_at;
use tak_proto::NodeInfo;

use support::{RemoteRecord, run_tak_expect_failure, write_remote_inventory, write_tasks};

#[test]
fn run_remote_timeout_reports_last_known_transport_state() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    let state_root = temp.path().join("state");
    write_tasks(
        temp.path(),
        "REMOTE = Remote(pool=\"build\", required_tags=[\"builder\"], required_capabilities=[\"linux\"], transport=TorOnionService(), runtime=ContainerRuntime(image=\"alpine:3.20\"))\nSPEC = module_spec(tasks=[task(\"check\", steps=[cmd(\"sh\", \"-c\", \"echo should-not-run\")], execution=RemoteOnly(REMOTE))])\nSPEC\n",
    )?;
    write_remote_inventory(
        &config_root,
        &[RemoteRecord {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://builder-a.onion".into(),
            bearer_token: "secret".into(),
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            enabled: true,
        }],
    )?;
    write_remote_observation_at(
        &state_root,
        &NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://builder-a.onion".into(),
            healthy: false,
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            transport_state: "recovering".into(),
            transport_detail: "rendezvous accept failed".into(),
        },
        1_734_000_000_000,
    )?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let dial_addr = listener.local_addr()?.to_string();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe");
        let mut request = [0_u8; 256];
        let _ = stream.read(&mut request);
        thread::sleep(Duration::from_millis(500));
    });
    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());
    env.insert("XDG_STATE_HOME".into(), state_root.display().to_string());
    env.insert("TAK_TEST_TOR_ONION_DIAL_ADDR".into(), dial_addr);
    env.insert("TAK_TEST_TOR_PROBE_TIMEOUT_MS".into(), "100".into());
    let (_, stderr) = run_tak_expect_failure(temp.path(), &["run", "--remote", "check"], &env)?;
    assert!(
        stderr.contains("Remote node info probe timed out for check"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("builder-a"), "stderr:\n{stderr}");
    assert!(stderr.contains("recovering"), "stderr:\n{stderr}");
    assert!(
        stderr.contains("rendezvous accept failed"),
        "stderr:\n{stderr}"
    );
    server.join().expect("probe server");
    Ok(())
}
