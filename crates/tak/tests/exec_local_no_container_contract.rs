//! Black-box contract for `tak exec --local-no-container`.

use crate::support::run_tak_output;

use std::collections::BTreeMap;

use anyhow::Result;

#[test]
fn exec_supports_local_no_container_host_execution() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let mut env = BTreeMap::new();
    env.insert("TAK_RUNTIME_SOURCE".to_string(), "none".to_string());
    let output = run_tak_output(
        temp.path(),
        &[
            "exec",
            "--local-no-container",
            "--",
            "sh",
            "-c",
            "printf '%s\\n' \"$TAK_RUNTIME_SOURCE\"",
        ],
        &env,
    )?;

    assert!(output.status.success(), "status: {:?}", output.status);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "none\n");
    Ok(())
}
