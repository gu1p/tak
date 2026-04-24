use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::live_direct::{
    LiveDirectRoots, init_direct_agent, spawn_direct_agent_with_env,
};
use crate::support::live_direct_remote::add_remote;
use crate::support::live_direct_token::wait_for_token;
use crate::support::tor_smoke::takd_bin;
use crate::support::{run_tak_expect_success, write_tasks};

#[test]
fn share_workspace_preserves_session_files_without_syncing_them_back() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = Runtime.Image("alpine:3.20")
SESSION = session("rust", execution=Execution.Local(runtime=RUNTIME), reuse=SessionReuse.Workspace())

SPEC = module_spec(
  sessions=[SESSION],
  tasks=[
    task("build", steps=[cmd("sh", "-c", "mkdir -p .session && echo cached > .session/build.txt")], execution=Execution.Session("rust")),
    task("test", deps=[":build"], outputs=[path("out")], steps=[cmd("sh", "-c", "test -f .session/build.txt && mkdir -p out && cat .session/build.txt > out/result.txt")], execution=Execution.Session("rust")),
  ],
)
SPEC
"#,
    )?;

    let mut env = BTreeMap::new();
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    let stdout = run_tak_expect_success(&workspace, &["run", "test"], &env)?;

    assert!(stdout.contains("session=rust"), "stdout:\n{stdout}");
    assert!(
        stdout.contains("reuse=share_workspace"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("out/result.txt"))?.trim(),
        "cached"
    );
    assert!(!workspace.join(".session/build.txt").exists());
    Ok(())
}

#[test]
fn remote_share_workspace_preserves_files_between_session_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());
    write_tasks(
        &workspace,
        r#"REMOTE = Execution.Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=Transport.DirectHttps(), runtime=Runtime.Image("alpine:3.20"))
SESSION = session("remote-rust", execution=REMOTE, reuse=SessionReuse.Workspace())

SPEC = module_spec(
  sessions=[SESSION],
  tasks=[
    task("build", steps=[cmd("sh", "-c", "mkdir -p .session && echo remote-cached > .session/build.txt")], execution=Execution.Session("remote-rust")),
    task("test", deps=[":build"], outputs=[path("out")], steps=[cmd("sh", "-c", "test -f .session/build.txt && mkdir -p out && cat .session/build.txt > out/remote-result.txt")], execution=Execution.Session("remote-rust")),
  ],
)
SPEC
"#,
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "session-remote-workspace");
    let serve_env = [("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string())];
    let _agent = spawn_direct_agent_with_env(&takd, &roots, &serve_env);
    let token = wait_for_token(&takd, &roots);
    add_remote(&workspace, &roots, &token);

    let mut env = BTreeMap::new();
    env.insert(
        "XDG_CONFIG_HOME".to_string(),
        roots.client_config_root.display().to_string(),
    );
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    let stdout = run_tak_expect_success(&workspace, &["run", "test"], &env)?;

    assert!(stdout.contains("session=remote-rust"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/remote-result.txt"))?.trim(),
        "remote-cached"
    );
    assert!(!workspace.join(".session/build.txt").exists());
    Ok(())
}
