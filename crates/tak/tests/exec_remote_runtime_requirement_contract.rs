//! Black-box contract for `tak exec --remote` runtime requirements.

use std::collections::BTreeMap;
use std::fs;

use anyhow::Result;

use crate::support::run_tak_output;

#[test]
fn exec_remote_requires_resolvable_container_runtime() -> Result<()> {
    fs::create_dir_all(".tmp")?;
    let temp = tempfile::tempdir_in(".tmp")?;
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace)?;

    let output = run_tak_output(
        &workspace,
        &["exec", "--remote", "--", "sh", "-c", "echo should-not-run"],
        &BTreeMap::new(),
    )?;

    assert!(
        !output.status.success(),
        "status unexpectedly succeeded: {:?}",
        output.status
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(
            "tak exec requires --container-image or --container-dockerfile for container execution"
        ),
        "stderr:\n{stderr}"
    );
    Ok(())
}
