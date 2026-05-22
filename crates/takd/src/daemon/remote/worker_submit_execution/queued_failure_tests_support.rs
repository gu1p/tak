use super::super::*;
use crate::daemon::remote::{RemoteNodeContext, SubmitAttemptStore};

#[path = "queued_failure_tests_admission.rs"]
mod admission_support;
#[path = "queued_failure_tests_payload.rs"]
mod payload_support;
#[path = "queued_failure_tests_store.rs"]
mod store_support;

pub(super) use admission_support::admit_resources;
pub(super) use payload_support::poison_status_state;
use payload_support::{payload, remote_context};
use store_support::register_submit;

pub(super) struct TestQueuedSubmit {
    pub(super) store: SubmitAttemptStore,
    pub(super) context: RemoteNodeContext,
    pub(super) idempotency_key: String,
    pub(super) execution_root_base: std::path::PathBuf,
    pub(super) cancellation: tak_runner::RunCancellation,
    pub(super) payload: RemoteWorkerSubmitPayload,
}

impl TestQueuedSubmit {
    pub(super) fn new(task_run_id: &str) -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let temp_path = temp.keep();
        let execution_root_base = temp_path.join("exec");
        let store =
            SubmitAttemptStore::with_db_path(temp_path.join("agent.sqlite")).expect("store");
        let context = remote_context();
        let payload = payload(task_run_id);
        let idempotency_key = register_submit(&store, &payload, &execution_root_base);
        let cancellation = context
            .register_active_execution(
                idempotency_key.clone(),
                &payload.task_run_id,
                payload.attempt,
            )
            .expect("register active execution");
        Self {
            store,
            context,
            idempotency_key,
            execution_root_base,
            cancellation,
            payload,
        }
    }

    pub(super) fn execution(&self) -> RemoteWorkerSubmitExecution {
        RemoteWorkerSubmitExecution {
            store: self.store.clone(),
            context: self.context.clone(),
            idempotency_key: self.idempotency_key.clone(),
            execution_root_base: self.execution_root_base.clone(),
            selected_node_id: "builder-a".into(),
            transport_kind: "direct".into(),
            image_cache: None,
            cancellation: self.cancellation.clone(),
            payload: self.payload.clone(),
            admission: PreparedResourceAdmission::Queued {
                queue_position: 1,
                queued_at_ms: 1,
            },
        }
    }
}

pub(super) fn assert_failed_and_unregistered(
    case: &TestQueuedSubmit,
    error: &str,
    task_run_id: &str,
) {
    let result = case
        .store
        .result_payload(&case.idempotency_key)
        .expect("result query")
        .expect("terminal result");
    assert!(result.contains(r#""success":false"#));
    assert!(result.contains(error));
    let events = case.store.events(&case.idempotency_key).expect("events");
    assert!(
        events
            .iter()
            .any(|event| { event.payload_json.contains(r#""kind":"TASK_FAILED""#) }),
        "missing TASK_FAILED event: {events:?}"
    );
    assert!(
        !case
            .context
            .cancel_active_task(task_run_id, Some(1))
            .unwrap()
    );
}
