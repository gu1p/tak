use crate::support;

use std::collections::BTreeMap;

use anyhow::Result;

use support::{RemoteRecord, run_tak_expect_failure, write_remote_inventory, write_tasks};

#[test]
fn run_remote_any_transport_lists_any_requirement_without_transport_mismatch_rejections()
-> Result<()> {
    let temp = tempfile::tempdir()?;
    let config_root = temp.path().join("config");
    write_tasks(
        temp.path(),
        r#"
REMOTE = Execution.Remote(
  pool="build",
  required_tags=["builder"],
  required_capabilities=["linux"],
  runtime=Runtime.Image("alpine:3.20"),
)

SPEC = module_spec(tasks=[
  task(
    "collect_remote_report",
    steps=[cmd("sh", "-c", "echo should-not-run")],
    execution=REMOTE,
  ),
])
SPEC
"#,
    )?;
    write_remote_inventory(
        &config_root,
        &[
            RemoteRecord {
                node_id: "builder-default".into(),
                display_name: "builder-default".into(),
                base_url: "http://builder-default".into(),
                bearer_token: "secret".into(),
                pools: vec!["default".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: "direct".into(),
                enabled: true,
            },
            RemoteRecord {
                node_id: "builder-tor".into(),
                display_name: "builder-tor".into(),
                base_url: "http://builder-tor.onion".into(),
                bearer_token: "secret".into(),
                pools: vec!["build".into()],
                tags: vec!["runner".into()],
                capabilities: vec!["linux".into()],
                transport: "tor".into(),
                enabled: true,
            },
        ],
    )?;

    let mut env = BTreeMap::new();
    env.insert("XDG_CONFIG_HOME".into(), config_root.display().to_string());

    let (_stdout, stderr) =
        run_tak_expect_failure(temp.path(), &["run", "//:collect_remote_report"], &env)?;

    for expected in [
        "Remote placement failed for //:collect_remote_report",
        "Required remote: pool=build tags=builder capabilities=linux transport=any",
        "builder-default",
        "pool mismatch: required build, remote pools=default",
        "builder-tor",
        "missing tags=builder",
    ] {
        assert!(stderr.contains(expected), "stderr:\n{stderr}");
    }
    assert!(
        !stderr.contains("transport mismatch:"),
        "stderr should not report transport mismatches for unconstrained remotes:\n{stderr}"
    );
    Ok(())
}
