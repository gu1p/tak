use anyhow::{Result, bail};
use tak_core::model::{BackoffDef, RetryDef};
use tak_proto::{FusedTaskMember, RetryBackoff, RetryPolicy, retry_backoff};

use super::super::RemoteWorkerFusedMember;
use super::parse_remote_worker_step;

pub(super) fn parse_remote_worker_fused_member(
    member: &FusedTaskMember,
) -> Result<RemoteWorkerFusedMember> {
    let task_label = member.task_label.trim().to_string();
    if task_label.is_empty() {
        bail!("invalid_submit_fields: fused_members.task_label is required");
    }
    Ok(RemoteWorkerFusedMember {
        task_label,
        steps: member
            .steps
            .iter()
            .map(parse_remote_worker_step)
            .collect::<Result<Vec<_>>>()?,
        timeout_s: member.timeout_s,
        retry: member
            .retry
            .as_ref()
            .map(parse_remote_worker_retry)
            .transpose()?
            .unwrap_or_default(),
        execution_label: member.execution_label.clone(),
    })
}

fn parse_remote_worker_retry(retry: &RetryPolicy) -> Result<RetryDef> {
    Ok(RetryDef {
        attempts: retry.attempts,
        on_exit: retry.on_exit.clone(),
        backoff: retry
            .backoff
            .as_ref()
            .map(parse_remote_worker_backoff)
            .transpose()?
            .unwrap_or_default(),
    })
}

fn parse_remote_worker_backoff(backoff: &RetryBackoff) -> Result<BackoffDef> {
    match backoff.kind.as_ref() {
        Some(retry_backoff::Kind::Fixed(fixed)) => Ok(BackoffDef::Fixed {
            seconds: fixed.seconds,
        }),
        Some(retry_backoff::Kind::ExpJitter(exp)) => Ok(BackoffDef::ExpJitter {
            min_s: exp.min_s,
            max_s: exp.max_s,
            jitter: exp.jitter.clone(),
        }),
        None => bail!("invalid_submit_fields: fused_members.retry.backoff.kind is required"),
    }
}
