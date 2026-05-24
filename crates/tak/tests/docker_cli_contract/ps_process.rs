use std::collections::BTreeMap;
use std::process::{Child, Command as StdCommand, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::support::{self, run_tak_output};

pub(super) fn spawn_tak_child(
    workspace_root: &std::path::Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
) -> Result<Child> {
    let mut command = StdCommand::new(support::tak_bin());
    command
        .current_dir(workspace_root)
        .args(args)
        .env("TAKD_SOCKET", workspace_root.join(".missing-takd.sock"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        command.env(key, value);
    }
    Ok(command.spawn()?)
}

pub(super) fn wait_for_docker_ps(
    workspace_root: &std::path::Path,
    args: &[&str],
    extra_env: &BTreeMap<String, String>,
    needle: &str,
) -> Result<String> {
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last_stdout = String::new();
    while Instant::now() < deadline {
        let output = run_tak_output(workspace_root, args, extra_env)?;
        assert!(output.status.success(), "status: {:?}", output.status);
        last_stdout = String::from_utf8_lossy(&output.stdout).to_string();
        if last_stdout.contains(needle) {
            return Ok(last_stdout);
        }
        thread::sleep(Duration::from_millis(100));
    }
    anyhow::bail!("timed out waiting for `{needle}` in docker ps output:\n{last_stdout}");
}

pub(super) struct ChildCleanup<'a>(pub(super) &'a mut Child);

impl Drop for ChildCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
