use prost::Message;
use tak_proto::RemoteEvent;

#[test]
fn remote_event_round_trips_optional_queue_position_on_tag_nine() {
    let encoded = RemoteEvent {
        seq: 7,
        kind: "TASK_QUEUE_POSITION".into(),
        queue_position: Some(3),
        ..RemoteEvent::default()
    }
    .encode_to_vec();

    let decoded = RemoteEvent::decode(encoded.as_slice()).expect("decode event");
    assert_eq!(decoded.queue_position, Some(3));
    assert!(encoded.windows(2).any(|pair| pair == [0x48, 0x03]));
}
