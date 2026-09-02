use tak_proto::worker_v2::{
    INCOMPLETE_PROCESS_OBSERVATIONS, WorkerProcessObservation, WorkerResources, WorkerSnapshot,
    bounded_process_observations, encode_snapshot,
};

#[test]
fn process_observations_bound_control_text_field_size_and_count() {
    let mut invalid_control = snapshot(vec![process("bad\nname", Vec::new())]);
    assert!(encode_snapshot(&invalid_control).is_err());

    invalid_control.processes[0].name = "process".into();
    invalid_control.processes[0].arguments = vec!["x".repeat(16 * 1_024 + 1)];
    assert!(encode_snapshot(&invalid_control).is_err());

    let too_many = snapshot(vec![process("p", Vec::new()); 4_097]);
    assert!(encode_snapshot(&too_many).is_err());
}

#[test]
fn bounded_process_observations_preserve_valid_values_without_a_marker() {
    let expected = vec![process("tool", vec!["--check".into()])];

    let bounded = bounded_process_observations(expected.clone(), 1_024);

    assert_eq!(bounded, expected);
    assert!(
        bounded
            .iter()
            .all(|process| process.name != INCOMPLETE_PROCESS_OBSERVATIONS)
    );
}

#[test]
fn invalid_and_over_budget_observations_produce_a_valid_bounded_marker() {
    let invalid = process("bad\nname", Vec::new());
    let bounded_invalid = bounded_process_observations(vec![invalid], 1_024);
    assert_eq!(bounded_invalid, vec![incomplete_marker()]);
    assert!(encode_snapshot(&snapshot(bounded_invalid)).is_ok());

    let over_budget = process("tool", vec!["x".repeat(80)]);
    let bounded_over_budget = bounded_process_observations(vec![over_budget], 64);
    assert_eq!(bounded_over_budget, vec![incomplete_marker()]);
    assert!(encode_snapshot(&snapshot(bounded_over_budget)).is_ok());
}

fn snapshot(processes: Vec<WorkerProcessObservation>) -> WorkerSnapshot {
    WorkerSnapshot {
        protocol_version: 2,
        node_id: "worker-a".into(),
        healthy: true,
        sampled_at_ms: 1,
        capacity: resources(1),
        usage: resources(0),
        queue_depth: 0,
        cached_content: Vec::new(),
        processes,
    }
}

fn process(name: &str, arguments: Vec<String>) -> WorkerProcessObservation {
    WorkerProcessObservation {
        name: name.into(),
        arguments,
    }
}

fn incomplete_marker() -> WorkerProcessObservation {
    process(INCOMPLETE_PROCESS_OBSERVATIONS, Vec::new())
}

fn resources(execution_slots: u32) -> WorkerResources {
    WorkerResources {
        cpu_millis: 1,
        memory_bytes: 1,
        execution_slots,
    }
}
