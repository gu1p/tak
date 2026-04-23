use crate::support;

use std::collections::BTreeMap;

use anyhow::Result;

use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::tor_smoke::takd_bin;
use support::{run_tak_expect_failure, write_tasks};

#[test]
fn run_command_remote_flag_requires_container_runtime_when_none_is_resolvable() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(tasks=[task("check", steps=[cmd("sh", "-c", "mkdir -p out && echo should-not-run > out/marker.txt")])])
SPEC
"#,
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "missing-runtime-builder");
    let _agent = spawn_direct_agent(&takd, &roots);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);

    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        roots.client_config_root.display().to_string(),
    );
    let (_stdout, stderr) =
        run_tak_expect_failure(&workspace_root, &["run", "--remote", "check"], &env)?;

    assert!(
        stderr.contains(
            "task //:check requires a containerized runtime for --remote; provide --container-image, --container-dockerfile, Remote(..., runtime=...), or TASKS.py defaults.container_runtime"
        ),
        "stderr:\n{stderr}"
    );
    assert!(!workspace_root.join("out/marker.txt").exists());
    Ok(())
}
