mod support;

use std::collections::BTreeMap;

use anyhow::Result;

use support::{
    RemoteRecord, run_tak_expect_failure, spawn_auth_rejecting_submit_server,
    spawn_timeout_node_info_server, write_remote_inventory, write_tasks,
};

#[test]
fn run_remote_auth_fallback_preserves_timeout_guidance() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    let remote_exec_root = temp.path().join("remote-exec");
    write_tasks(
        temp.path(),
        "REMOTE = Remote(pool=\"build\", required_tags=[\"builder\"], required_capabilities=[\"linux\"], transport=DirectHttps())\nSPEC = module_spec(tasks=[task(\"check\", steps=[cmd(\"sh\", \"-c\", \"echo should-not-run\")], execution=RemoteOnly(REMOTE))])\nSPEC\n",
    )?;

    let (auth_base_url, auth_handle) = spawn_auth_rejecting_submit_server("builder-auth-fail");
    let (timeout_base_url, timeout_handle) = spawn_timeout_node_info_server("builder-timeout");
    write_remote_inventory(
        &config_root,
        &[
            RemoteRecord {
                node_id: "builder-auth-fail".into(),
                display_name: "builder-auth-fail".into(),
                base_url: auth_base_url,
                bearer_token: "secret".into(),
                pools: vec!["build".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: "direct".into(),
                enabled: true,
            },
            RemoteRecord {
                node_id: "builder-timeout".into(),
                display_name: "builder-timeout".into(),
                base_url: timeout_base_url,
                bearer_token: "secret".into(),
                pools: vec!["build".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: "direct".into(),
                enabled: true,
            },
        ],
    )?;

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());
    env.insert(
        "TAKD_REMOTE_EXEC_ROOT".into(),
        remote_exec_root.display().to_string(),
    );

    let (_, stderr) = run_tak_expect_failure(temp.path(), &["run", "--remote", "check"], &env)?;
    assert!(
        stderr.contains("Remote node info probe timed out for check"),
        "stderr:\n{stderr}"
    );
    assert!(stderr.contains("builder-timeout"), "stderr:\n{stderr}");
    assert!(
        stderr.contains("auth failed during submit with HTTP 401"),
        "stderr:\n{stderr}"
    );

    auth_handle.join().expect("auth server");
    timeout_handle.join().expect("timeout server");
    Ok(())
}
