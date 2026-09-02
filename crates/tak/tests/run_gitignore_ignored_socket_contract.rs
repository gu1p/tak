//! Black-box contract for ignored special entries during daemon-owned submission.

use crate::support;

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::net::UnixListener;

use anyhow::Result;

#[test]
fn run_skips_an_unselected_gitignored_socket_before_contacting_takd() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("src"))?;
    fs::create_dir_all(workspace.join(".tmp/daemon"))?;
    fs::write(workspace.join(".gitignore"), ".tmp\n")?;
    fs::write(workspace.join("src/input.txt"), "visible\n")?;
    let socket = workspace.join(".tmp/daemon/agent-control.sock");
    let _socket = UnixListener::bind(support::unix_socket_bind_path::short_bind_path(&socket))?;
    support::write_tasks(
        &workspace,
        r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "check",
  context=CurrentState(ignored=[gitignore()], include=[path("src")]),
  steps=[cmd("true")],
)])
SPEC
"#,
    )?;

    let output = support::run_tak_output(&workspace, &["run", "check"], &BTreeMap::new())?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "missing takd should fail");
    assert!(
        stderr.contains("Local takd is unavailable"),
        "stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("unsupported workspace entry"),
        "ignored socket reached workspace manifest construction:\n{stderr}"
    );
    Ok(())
}
