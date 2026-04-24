use crate::support;

use std::collections::BTreeMap;

use anyhow::Result;

use support::{RemoteRecord, run_tak_expect_failure, write_remote_inventory, write_tasks};

#[test]
fn run_command_remote_flag_keeps_existing_remote_requirements() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    write_tasks(
        temp.path(),
        r#"
REMOTE = Execution.Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  transport=Transport.TorOnionService(),
  runtime=Runtime.Image("alpine:3.20"),
)

SPEC = module_spec(tasks=[task(
  "check",
  steps=[cmd("sh", "-c", "echo should-not-run")],
  execution=REMOTE,
)])
SPEC
"#,
    )?;
    write_remote_inventory(
        &config_root,
        &[RemoteRecord {
            node_id: "builder-direct".into(),
            display_name: "builder-direct".into(),
            base_url: "not-a-url".into(),
            bearer_token: "secret".into(),
            pools: vec!["build".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            enabled: true,
        }],
    )?;

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());

    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["run", "--remote", "check"], &env)?;

    assert!(
        stderr.contains("Remote placement failed for //:check"),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains(concat!(
            "Required remote: pool=build ",
            "tags=builder capabilities=linux transport=tor",
        )),
        "stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("transport mismatch: required tor, remote transport=direct"),
        "stderr:\n{stderr}"
    );
    Ok(())
}
