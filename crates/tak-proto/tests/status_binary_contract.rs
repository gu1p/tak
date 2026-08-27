use prost::Message;
use tak_proto::NodeStatusResponse;

mod support;

use support::sample_status;

#[test]
fn node_status_messages_round_trip_as_binary() {
    let status = sample_status();
    let encoded = status.encode_to_vec();
    let decoded = NodeStatusResponse::decode(encoded.as_slice()).expect("decode node status");
    let node = decoded.node.expect("node");
    assert_eq!(node.node_id, "builder-a");
    assert_eq!(node.transport_state, "ready");
    assert_eq!(decoded.active_jobs.len(), 1);
    assert_eq!(decoded.active_jobs[0].task_label, "//apps/web:build");
    let active_label = decoded.active_jobs[0].execution_label.as_deref();
    assert_eq!(active_label, Some("check.build"));
    assert_eq!(decoded.queued_jobs.len(), 1);
    assert_eq!(decoded.queued_jobs[0].queue_position, 1);
    let queued_label = decoded.queued_jobs[0].execution_label.as_deref();
    assert_eq!(queued_label, Some("check.test"));
    let cpu = decoded.cpu.expect("cpu");
    assert_eq!(cpu.tak_reserved_cores, Some(2.0));
    let memory = decoded.memory.expect("memory");
    assert_eq!(memory.tak_reserved_bytes, Some(2_048));
    let envelope = decoded.resource_envelope.expect("resource envelope");
    assert_eq!(envelope.workload_memory_bytes, 6_144);
    assert_eq!(envelope.admittable_memory_bytes, 3_072);
    let pressure = decoded.resource_pressure.expect("resource pressure");
    assert_eq!(pressure.state, "healthy");
    assert_eq!(pressure.healthy_samples, 4);
}
