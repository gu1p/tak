//! Black-box contract for `tak exec --remote` runtime requirements.

use crate::support;

use anyhow::Result;

use support::direct_remote_runtime::{client_env, start_direct_agent};
use support::run_tak_output;

#[test]
fn exec_remote_requires_resolvable_container_runtime() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let _agent = start_direct_agent(temp.path(), temp.path(), "exec-remote-runtime-required");

    let output = run_tak_output(
        temp.path(),
        &["exec", "--remote", "--", "sh", "-c", "echo should-not-run"],
        &client_env(temp.path()),
    )?;

    assert!(
        !output.status.success(),
        "status unexpectedly succeeded: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "task //:exec requires a containerized runtime for --remote; provide --container-image, --container-dockerfile, Remote(..., runtime=...), or TASKS.py defaults.container_runtime"
        ),
        "stderr:\n{stderr}"
    );
    Ok(())
}
