//! Black-box contract for remote run-mode overrides.

use crate::support;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent_with_env};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::tor_smoke::takd_bin;
use support::{run_tak_expect_success, write_tasks};

#[test]
fn run_command_executes_neutral_task_remotely_with_remote_flag() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());
    let marker = workspace_root.join("out/remote-run.txt");
    write_tasks(
        &workspace_root,
        "SPEC = module_spec(tasks=[task(\"check\", outputs=[path(\"out\")], steps=[cmd(\"sh\", \"-c\", \"mkdir -p out && echo remote-run > out/remote-run.txt\")])])\nSPEC\n",
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "override-remote-builder");
    let _agent = spawn_direct_agent_with_env(&takd, &roots, &[]);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace_root, &roots, &token);

    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        roots.client_config_root.display().to_string(),
    );
    let stdout = run_tak_expect_success(&workspace_root, &["run", "--remote", "check"], &env)?;

    assert!(stdout.contains("placement=remote"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("remote_node=override-remote-builder"),
        "stdout:\n{stdout}"
    );
    assert_eq!(fs::read_to_string(marker)?.trim(), "remote-run");
    Ok(())
}
