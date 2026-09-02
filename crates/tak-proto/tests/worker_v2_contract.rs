use tak_proto::worker_v2::{
    WorkerIdentity, WorkerProcessObservation, WorkerResources, WorkerSnapshot,
    decode_display_payload, decode_identity, decode_snapshot, encode_display_payload,
    encode_identity, encode_snapshot,
};

#[test]
fn worker_identity_is_a_strict_protocol_v2_onboarding_dto() {
    let identity = WorkerIdentity {
        protocol_version: 2,
        node_id: "worker-a".into(),
        display_name: "Worker A".into(),
        base_url: "http://worker-a.onion".into(),
        pools: vec!["build".into()],
        tags: vec!["linux".into()],
        capabilities: vec!["docker".into()],
        transport: "tor".into(),
    };
    assert_eq!(
        decode_identity(&encode_identity(&identity).unwrap()).unwrap(),
        identity
    );

    let v1 = br#"{"protocol_version":1,"node_id":"worker-a","display_name":"Worker A","base_url":"http://worker-a.onion","pools":[],"tags":[],"capabilities":[],"transport":"tor"}"#;
    assert!(decode_identity(v1).is_err());
}

#[test]
fn legacy_display_bytes_are_only_carried_in_a_strict_v2_envelope() {
    let encoded = encode_display_payload(b"protobuf-or-text").unwrap();
    assert_eq!(
        decode_display_payload(&encoded).unwrap(),
        b"protobuf-or-text"
    );
    assert!(
        decode_display_payload(br#"{"protocol_version":1,"payload_base64":"cHJvdG9idWY="}"#)
            .is_err()
    );
    assert!(
        decode_display_payload(br#"{"protocol_version":2,"payload_base64":"not-base64"}"#).is_err()
    );
}

#[test]
fn worker_snapshot_requires_exact_protocol_v2_and_strict_resources() {
    let snapshot = WorkerSnapshot {
        protocol_version: 2,
        node_id: "worker-a".into(),
        healthy: true,
        sampled_at_ms: 42,
        capacity: WorkerResources {
            cpu_millis: 8_000,
            memory_bytes: 16_000,
            execution_slots: 8,
        },
        usage: WorkerResources {
            cpu_millis: 1_000,
            memory_bytes: 4_000,
            execution_slots: 2,
        },
        queue_depth: 1,
        cached_content: vec!["sha256:base".into()],
        processes: vec![WorkerProcessObservation {
            name: "rustc".into(),
            arguments: vec!["rustc".into(), "--crate-name".into(), "demo".into()],
        }],
    };
    assert_eq!(
        decode_snapshot(&encode_snapshot(&snapshot).unwrap()).unwrap(),
        snapshot
    );

    for invalid in [
        r#"{"protocol_version":1,"node_id":"worker-a","healthy":true,"sampled_at_ms":1,"capacity":{"cpu_millis":1,"memory_bytes":1,"execution_slots":1},"usage":{"cpu_millis":0,"memory_bytes":0,"execution_slots":0},"queue_depth":0,"cached_content":[],"processes":[]}"#,
        r#"{"protocol_version":2,"node_id":"","healthy":true,"sampled_at_ms":1,"capacity":{"cpu_millis":1,"memory_bytes":1,"execution_slots":1},"usage":{"cpu_millis":0,"memory_bytes":0,"execution_slots":0},"queue_depth":0,"cached_content":[],"processes":[]}"#,
        r#"{"protocol_version":2,"node_id":"worker-a","healthy":true,"sampled_at_ms":1,"capacity":{"cpu_millis":1,"memory_bytes":1,"execution_slots":1},"usage":{"cpu_millis":2,"memory_bytes":0,"execution_slots":0},"queue_depth":0,"cached_content":[],"processes":[]}"#,
    ] {
        assert!(
            decode_snapshot(invalid.as_bytes()).is_err(),
            "accepted {invalid}"
        );
    }
}
