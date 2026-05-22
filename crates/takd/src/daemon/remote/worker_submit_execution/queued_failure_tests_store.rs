use std::path::Path;

use super::super::*;
use crate::daemon::remote::{SubmitAttemptStore, SubmitRegistration};

pub(crate) fn register_submit(
    store: &SubmitAttemptStore,
    payload: &RemoteWorkerSubmitPayload,
    execution_root_base: &Path,
) -> String {
    match store
        .register_submit_with_execution_root_base(
            &payload.task_run_id,
            Some(payload.attempt),
            &payload.task_label,
            payload.execution_label.as_deref(),
            "builder-a",
            execution_root_base,
        )
        .expect("register submit")
    {
        SubmitRegistration::Created { idempotency_key }
        | SubmitRegistration::Attached { idempotency_key } => idempotency_key,
    }
}
