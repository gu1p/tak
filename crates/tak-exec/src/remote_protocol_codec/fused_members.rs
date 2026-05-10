use anyhow::Result;
use tak_core::model::{BackoffDef, ResolvedTask, RetryDef};
use tak_proto::{
    ExpJitterRetryBackoff, FixedRetryBackoff, FusedTaskMember, RetryBackoff, RetryPolicy,
    retry_backoff,
};

pub(super) fn fused_member_submit_value(task: &ResolvedTask) -> Result<FusedTaskMember> {
    Ok(FusedTaskMember {
        task_label: task.label.to_string(),
        steps: task
            .steps
            .iter()
            .map(super::step_submit_value)
            .collect::<Result<Vec<_>>>()?,
        timeout_s: task.timeout_s,
        retry: Some(retry_submit_value(&task.retry)),
    })
}

fn retry_submit_value(retry: &RetryDef) -> RetryPolicy {
    RetryPolicy {
        attempts: retry.attempts,
        on_exit: retry.on_exit.clone(),
        backoff: Some(RetryBackoff {
            kind: Some(match &retry.backoff {
                BackoffDef::Fixed { seconds } => {
                    retry_backoff::Kind::Fixed(FixedRetryBackoff { seconds: *seconds })
                }
                BackoffDef::ExpJitter {
                    min_s,
                    max_s,
                    jitter,
                } => retry_backoff::Kind::ExpJitter(ExpJitterRetryBackoff {
                    min_s: *min_s,
                    max_s: *max_s,
                    jitter: jitter.clone(),
                }),
            }),
        }),
    }
}
