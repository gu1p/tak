use serde_json::json;
use tak_core::model::{
    BackoffDef, Hold, LocalDef, ModuleSpec, NeedDef, QueueDef, QueueDiscipline, QueueUseDef,
    RemoteDef, RemoteSelectionDef, RetryDef, StepDef, TaskLabel,
};

#[test]
fn module_and_local_execution_defaults_follow_contract() {
    let module: ModuleSpec = serde_json::from_value(json!({"tasks": []})).expect("module spec");
    let local: LocalDef = serde_json::from_value(json!({"id": "local"})).expect("local spec");
    assert_eq!(module.spec_version, 1);
    assert_eq!(local.max_parallel_tasks, 1);
    assert!(matches!(
        StepDef::default(),
        StepDef::Cmd { argv, cwd: None, env } if argv.is_empty() && env.is_empty()
    ));
}

#[test]
fn remote_execution_defaults_to_sequential_selection() {
    let remote: RemoteDef = serde_json::from_value(json!({})).expect("remote spec");
    assert!(matches!(remote.selection, RemoteSelectionDef::Sequential));
}

#[test]
fn need_and_queue_defaults_apply_when_optional_fields_are_omitted() {
    let need: NeedDef = serde_json::from_value(json!({
        "limiter": {"name": "cpu", "scope": "machine"}
    }))
    .expect("need");
    let queue: QueueUseDef = serde_json::from_value(json!({
        "queue": {"name": "build", "scope": "machine"}
    }))
    .expect("queue");

    assert_eq!(need.slots, 1.0);
    assert!(matches!(need.hold, Hold::During));
    assert_eq!(queue.slots, 1);
    assert_eq!(queue.priority, 0);
}

#[test]
fn retry_defaults_use_single_attempt_and_zero_delay_backoff() {
    let retry: RetryDef = serde_json::from_value(json!({})).expect("retry");
    assert_eq!(retry.attempts, 1);
    assert!(retry.on_exit.is_empty());
    assert!(matches!(retry.backoff, BackoffDef::Fixed { seconds } if seconds == 0.0));
}

#[test]
fn queue_and_backoff_defaults_fill_in_optional_fields() {
    let queue: QueueDef = serde_json::from_value(json!({
        "name": "build",
        "scope": "machine",
        "slots": 2
    }))
    .expect("queue def");
    let backoff: BackoffDef = serde_json::from_value(json!({
        "kind": "exp_jitter",
        "min_s": 1.0,
        "max_s": 2.0
    }))
    .expect("backoff");

    assert!(matches!(queue.discipline, QueueDiscipline::Fifo));
    assert_eq!(queue.max_pending, None);
    assert!(matches!(backoff, BackoffDef::ExpJitter { jitter, .. } if jitter == "full"));
}

#[test]
fn task_label_display_preserves_explicit_package_prefixes() {
    let label = TaskLabel {
        package: "pkg".into(),
        name: "build".into(),
    };
    assert_eq!(label.to_string(), "pkg:build");
}
