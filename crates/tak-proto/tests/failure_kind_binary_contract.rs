use prost::Message;
use tak_proto::{GetTaskResultResponse, RemoteFailureKind};

#[test]
fn confirmed_container_oom_round_trips_as_its_own_wire_kind() {
    let response = GetTaskResultResponse {
        success: false,
        exit_code: Some(137),
        failure_kind: Some(RemoteFailureKind::ContainerOom as i32),
        ..GetTaskResultResponse::default()
    };

    let encoded = response.encode_to_vec();
    let decoded = GetTaskResultResponse::decode(encoded.as_slice()).expect("decode result");

    assert_eq!(
        decoded.failure_kind,
        Some(RemoteFailureKind::ContainerOom as i32)
    );
}

#[test]
fn failure_kind_discriminants_remain_legacy_compatible() {
    assert_eq!(RemoteFailureKind::Unspecified as i32, 0);
    assert_eq!(RemoteFailureKind::Task as i32, 1);
    assert_eq!(RemoteFailureKind::Infrastructure as i32, 2);
    assert_eq!(RemoteFailureKind::Cancellation as i32, 3);
    assert_eq!(RemoteFailureKind::ResourceCapacity as i32, 4);
    assert_eq!(RemoteFailureKind::Unknown as i32, 5);
    assert_eq!(RemoteFailureKind::ContainerOom as i32, 6);
}
