//! Black-box contract for gitignore-aware remote context staging.

use crate::support;

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use support::live_direct::{LiveDirectRoots, init_direct_agent, spawn_direct_agent};
use support::live_direct_remote::add_remote;
use support::live_direct_token::wait_for_token;
use support::tor_smoke::takd_bin;
use support::{run_tak_expect_success, write_tasks};

#[test]
fn run_remote_context_uses_gitignore_and_readds_included_subtree() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let workspace_root = temp.path().join("workspace");
    let roots = LiveDirectRoots::new(temp.path());

    fs::create_dir_all(workspace_root.join("src"))?;
    fs::create_dir_all(workspace_root.join("target/private"))?;
    fs::create_dir_all(workspace_root.join("target/reinclude"))?;
    fs::write(workspace_root.join(".gitignore"), "target/\n")?;
    fs::write(workspace_root.join("src/input.txt"), "visible\n")?;
    fs::write(
        workspace_root.join("target/private/ignored.txt"),
        "should stay local\n",
    )?;
    fs::write(workspace_root.join("target/reinclude/one.txt"), "one\n")?;
    fs::write(workspace_root.join("target/reinclude/two.txt"), "two\n")?;
    write_tasks(
        &workspace_root,
        r#"SPEC = module_spec(defaults=Defaults(container_runtime=Runtime.Image("alpine:3.20")), tasks=[task(
  "check",
  context=CurrentState(
    ignored=[gitignore()],
    include=[path("target/reinclude")],
  ),
  outputs=[path("out")],
  steps=[cmd("sh", "-ceu", """
test -f src/input.txt
test -f target/reinclude/one.txt
test -f target/reinclude/two.txt
test ! -e target/private/ignored.txt
mkdir -p out
find target -type f | LC_ALL=C sort > out/files.txt
""")],
)])
SPEC
"#,
    )?;

    let takd = takd_bin();
    init_direct_agent(&takd, &roots, "gitignore-builder");
    let _agent = spawn_direct_agent(&takd, &roots);
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
        stdout.contains("remote_node=gitignore-builder"),
        "stdout:\n{stdout}"
    );
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/files.txt"))?,
        "target/reinclude/one.txt\ntarget/reinclude/two.txt\n"
    );
    Ok(())
}
