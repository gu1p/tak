use crate::support;

use support::remote_add::run_add_script;
use support::remote_daemon_v2::{FakeRemoteDaemon, remote};

#[test]
fn remote_add_token_or_location_invalid_input_stays_open_and_can_be_corrected() {
    let temp = tempfile::tempdir().expect("tempdir");
    let config_root = temp.path().join("config");
    let invite = "takd:tor:secret-invite";
    let daemon = FakeRemoteDaemon::spawn(
        temp.path(),
        vec![
            serde_json::json!({
                "type": "RemotePreview",
                "remote": remote("builder-location-retry-tui")
            }),
            serde_json::json!({
                "type": "RemoteAdded",
                "remote": remote("builder-location-retry-tui")
            }),
        ],
    );

    let output = run_add_script(
        &config_root,
        &format!("down,enter,paste:http://127.0.0.1:3000,enter,ctrl_u,paste:{invite},enter,enter"),
        &[("TAKD_SOCKET", daemon.socket().display().to_string())],
    )
    .expect("run scripted add");

    assert!(
        output.status.success(),
        "tak remote add should recover from invalid location\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("paste a takd token or secret Tor invite/address"),
        "missing inline validation:\n{stdout}"
    );
    assert!(
        stdout.contains("added remote builder-location-retry-tui"),
        "missing success after correction:\n{stdout}"
    );
    let requests = daemon.finish();
    assert_eq!(requests.len(), 2, "invalid input must not reach takd");
    assert_eq!(requests[0]["operation"]["type"], "PreviewRemote");
    assert_eq!(requests[0]["operation"]["invite"], invite);
    assert_eq!(requests[1]["operation"]["type"], "AddRemote");
    assert_eq!(requests[1]["operation"]["invite"], invite);
}
