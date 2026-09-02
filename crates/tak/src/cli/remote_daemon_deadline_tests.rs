use std::time::Duration;

use tak_proto::local_daemon::v2::Operation;

use super::remote_daemon::network_timeout;

#[test]
fn network_backed_remote_operations_allow_the_full_daemon_probe_envelope() {
    let timeout = Duration::from_secs(300);
    let operations = [
        Operation::PreviewRemote { invite: "i".into() },
        Operation::AddRemote { invite: "i".into() },
        Operation::GetRemoteStatus { node_ids: vec![] },
        Operation::ReadRemote {
            node_id: "n".into(),
            path: "/v2/worker/status".into(),
        },
    ];

    for operation in operations {
        assert_eq!(network_timeout(&operation), Some(timeout));
    }
}

#[test]
fn local_inventory_operations_keep_the_fast_default_deadline() {
    assert_eq!(network_timeout(&Operation::ListRemotes {}), None);
    assert_eq!(
        network_timeout(&Operation::RemoveRemote {
            node_id: "n".into(),
        }),
        None
    );
}
