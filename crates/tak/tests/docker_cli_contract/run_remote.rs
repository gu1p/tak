use std::collections::BTreeMap;

use anyhow::Result;

use crate::support::remote_daemon_v2::{FakeRemoteDaemon, remote};
use crate::support::run_tak_output;

#[test]
fn remote_list_prints_generated_alias_for_node_selection() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let daemon = FakeRemoteDaemon::spawn(
        temp.path(),
        vec![serde_json::json!({
            "type": "RemoteList",
            "remotes": [remote("builder-node-123456")]
        })],
    );

    let env = BTreeMap::from([("TAKD_SOCKET".into(), daemon.socket().display().to_string())]);
    let output = run_tak_output(temp.path(), &["remote", "list"], &env)?;
    assert!(output.status.success(), "status: {:?}", output.status);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("alias="), "stdout:\n{stdout}");
    assert!(stdout.contains("builder-node-123456"), "stdout:\n{stdout}");
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "ListRemotes");
    Ok(())
}
