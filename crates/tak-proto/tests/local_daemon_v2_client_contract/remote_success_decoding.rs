use tak_proto::local_daemon::v2::{Response, decode_response};

#[test]
fn remote_inventory_responses_are_typed_and_never_accept_bearer_tokens() {
    let raw = br#"{"protocol_version":2,"type":"RemoteList","request_id":"list-remotes","remotes":[{"node_id":"builder-a","display_name":"Builder A","base_url":"http://builder-a.onion","pools":["build"],"tags":["linux"],"capabilities":["docker"],"transport":"tor","enabled":true}]}"#;
    let Response::RemoteList { remotes, .. } =
        decode_response(raw, "list-remotes").expect("decode inventory")
    else {
        panic!("expected remote list")
    };
    assert_eq!(remotes[0].node_id, "builder-a");

    let leaked = br#"{"protocol_version":2,"type":"RemoteList","request_id":"list-remotes","remotes":[{"node_id":"builder-a","display_name":"Builder A","base_url":"http://builder-a.onion","bearer_token":"secret","pools":[],"tags":[],"capabilities":[],"transport":"tor","enabled":true}]}"#;
    assert!(decode_response(leaked, "list-remotes").is_err());
}

#[test]
fn remote_health_and_forwarded_reads_decode_as_protocol_v2_payloads() {
    let status = br#"{"protocol_version":2,"type":"RemoteStatus","request_id":"remote-status","remotes":[{"remote":{"node_id":"builder-a","display_name":"Builder A","base_url":"http://127.0.0.1:9","pools":[],"tags":[],"capabilities":[],"transport":"direct","enabled":true},"snapshot":{"protocol_version":2,"node_id":"builder-a","healthy":true,"sampled_at_ms":1,"capacity":{"cpu_millis":8000,"memory_bytes":16000,"execution_slots":8},"usage":{"cpu_millis":1000,"memory_bytes":4000,"execution_slots":2},"queue_depth":1,"cached_content":[],"processes":[]},"detail_base64":"CgsKCWJ1aWxkZXItYQ==","error":null,"peer":null}]}"#;
    assert!(matches!(
        decode_response(status, "remote-status").expect("decode status"),
        Response::RemoteStatus { remotes, .. } if remotes.len() == 1
    ));

    let bytes = br#"{"protocol_version":2,"type":"RemoteRead","request_id":"read","node_id":"builder-a","http_status":200,"body_base64":"bG9nCg=="}"#;
    assert!(matches!(
        decode_response(bytes, "read").expect("decode read"),
        Response::RemoteRead {
            http_status: 200,
            ..
        }
    ));
}
