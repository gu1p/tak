#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;

use tak_proto::encode_tor_invite_with_bearer;

use crate::support::raw_local_protocol::RawLocalProtocol;

#[path = "local_protocol_v2_remote_inventory_integration/worker.rs"]
mod worker;

const ONION: &str = "pg6mmjiyjmcrsslvykfwnntlaru7p5svn6y2ymmju6nubxndf4pscryd.onion";

#[tokio::test]
async fn daemon_onboards_lists_and_removes_remote_with_owner_only_inventory() {
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let root = tempfile::tempdir_in(".tmp").expect("temp root");
    let current = std::env::current_dir().expect("current test directory");
    let root_path = root
        .path()
        .strip_prefix(current)
        .expect("test temp root below current directory");
    let (dial, node_server) = worker::spawn(ONION).await;
    let mut daemon = RawLocalProtocol::start_with_remote_inventory(
        root_path,
        takd::TorBroker::for_direct_dial(dial),
    )
    .await;
    let invite =
        encode_tor_invite_with_bearer(&format!("http://{ONION}"), "invite-secret").unwrap();

    let added = daemon.exchange(&format!(r#"{{"protocol_version":2,"request_id":"add","operation":{{"type":"AddRemote","invite":"{invite}"}}}}"#)).await;
    assert!(added.contains(r#""type":"RemoteAdded""#), "{added}");
    assert!(!added.contains("bearer_token"));
    let path = root_path.join("config/tak/remotes.toml");
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );

    let listed = daemon
        .exchange(
            r#"{"protocol_version":2,"request_id":"list","operation":{"type":"ListRemotes"}}"#,
        )
        .await;
    assert!(listed.contains("builder-a"));
    assert!(!listed.contains("bearer_token"));
    let removed = daemon.exchange(r#"{"protocol_version":2,"request_id":"remove","operation":{"type":"RemoveRemote","node_id":"builder-a"}}"#).await;
    assert!(removed.contains(r#""removed":true"#));
    node_server.await.unwrap();
}
