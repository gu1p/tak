use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::run_tak_expect_failure;

#[test]
fn docker_ps_rejects_invalid_transport_selector() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let (_stdout, stderr) = run_tak_expect_failure(
        temp.path(),
        &["--transport", "driect", "docker", "ps"],
        &BTreeMap::new(),
    )?;

    assert!(stderr.contains("invalid value"), "stderr:\n{stderr}");
    assert!(stderr.contains("direct"), "stderr:\n{stderr}");
    assert!(stderr.contains("tor"), "stderr:\n{stderr}");
    assert!(stderr.contains("any"), "stderr:\n{stderr}");
    Ok(())
}
