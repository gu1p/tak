use serde_json::{Value, json};
use tak_proto::local_daemon::v2::{
    DecodeOutcome, Operation, Request, decode_request, encode_request,
};

#[test]
fn encoder_round_trips_daemon_owned_remote_management_operations() {
    let cases = [
        (
            Operation::PreviewRemote {
                invite: "takd:tor:secret-invite".into(),
            },
            json!({"type": "PreviewRemote", "invite": "takd:tor:secret-invite"}),
        ),
        (
            Operation::AddRemote {
                invite: "takd:tor:secret-invite".into(),
            },
            json!({"type": "AddRemote", "invite": "takd:tor:secret-invite"}),
        ),
        (Operation::ListRemotes {}, json!({"type": "ListRemotes"})),
        (
            Operation::RemoveRemote {
                node_id: "builder-a".into(),
            },
            json!({"type": "RemoveRemote", "node_id": "builder-a"}),
        ),
        (
            Operation::GetRemoteStatus {
                node_ids: vec!["builder-a".into()],
            },
            json!({"type": "GetRemoteStatus", "node_ids": ["builder-a"]}),
        ),
        (
            Operation::ReadRemote {
                node_id: "builder-a".into(),
                path: "/v2/worker/logs?lines=20".into(),
            },
            json!({"type": "ReadRemote", "node_id": "builder-a", "path": "/v2/worker/logs?lines=20"}),
        ),
    ];

    for (operation, expected) in cases {
        let request = Request {
            request_id: "remote-management".into(),
            operation,
        };
        let encoded = encode_request(&request).expect("encode remote operation");
        let value: Value = serde_json::from_str(&encoded).expect("request json");
        assert_eq!(value["operation"], expected);
        assert!(matches!(
            decode_request(&encoded).expect("server accepts client request"),
            DecodeOutcome::V2(decoded) if decoded == request
        ));
    }
}
