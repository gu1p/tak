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
fn share_paths_preserves_only_declared_paths_between_session_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"RUNTIME = ContainerRuntime(image="alpine:3.20")
SESSION = session(
  "cargo",
  execution=LocalOnly(Local("local", runtime=RUNTIME)),
  reuse=SharePaths([path("target")]),
)

SPEC = module_spec(
  sessions=[SESSION],
  tasks=[
    task("compile", steps=[cmd("sh", "-c", "mkdir -p target scratch && echo cached > target/cache.txt && echo leak > scratch/leak.txt")], execution=UseSession("cargo")),
    task("check", deps=[":compile"], outputs=[path("out")], steps=[cmd("sh", "-c", "test -f target/cache.txt && test ! -e scratch/leak.txt && mkdir -p out && cat target/cache.txt > out/cache.txt")], execution=UseSession("cargo")),
  ],
)
SPEC
"#,
    )?;

    let mut env = BTreeMap::new();
    env.insert("TAK_TEST_HOST_PLATFORM".to_string(), "other".to_string());
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(stdout.contains("session=cargo"), "stdout:\n{stdout}");
    assert!(stdout.contains("reuse=share_paths"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/cache.txt"))?.trim(),
        "cached"
    );
    assert!(!workspace.join("target/cache.txt").exists());
    assert!(!workspace.join("scratch/leak.txt").exists());
    Ok(())
}

#[test]
fn remote_share_paths_preserves_only_declared_paths_between_session_tasks() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());
    write_tasks(
        &workspace,
        r#"REMOTE = Remote(pool="build", required_tags=["builder"], required_capabilities=["linux"], transport=DirectHttps(), runtime=ContainerRuntime(image="alpine:3.20"))
SESSION = session("remote-cargo", execution=RemoteOnly(REMOTE), reuse=SharePaths([path("target")]))

SPEC = module_spec(
  sessions=[SESSION],
  tasks=[
    task("compile", steps=[cmd("sh", "-c", "mkdir -p target scratch && echo remote-cached > target/cache.txt && echo leak > scratch/leak.txt")], execution=UseSession("remote-cargo")),
    task("check", deps=[":compile"], outputs=[path("out")], steps=[cmd("sh", "-c", "test -f target/cache.txt && test ! -e scratch/leak.txt && mkdir -p out && cat target/cache.txt > out/remote-cache.txt")], execution=UseSession("remote-cargo")),
  ],
)
SPEC
"#,
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "session-remote-paths");
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
    let stdout = run_tak_expect_success(&workspace, &["run", "check"], &env)?;

    assert!(stdout.contains("session=remote-cargo"), "stdout:\n{stdout}");
    assert_eq!(
        fs::read_to_string(workspace.join("out/remote-cache.txt"))?.trim(),
        "remote-cached"
    );
    assert!(!workspace.join("target/cache.txt").exists());
    Ok(())
}
