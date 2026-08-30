use std::num::{NonZeroU32, NonZeroU64};

use anyhow::{Result, bail};
use tak_core::v2::{
    AuthoredLimiterClaim, AuthoredLimiterDefinition, AuthoredQueueDefinition, AuthoredQueueUse,
    DefinitionScope, HoldMode, QueueDiscipline, RetryPolicy,
};

use super::super::v2_wire as wire;

pub(super) fn queue_use(value: wire::QueueUse) -> Result<AuthoredQueueUse> {
    let slots = u32::try_from(value.slots)
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| anyhow::anyhow!("queue-use slots must be positive"))?;
    Ok(AuthoredQueueUse {
        name: value.queue.name,
        scope: scope(&value.queue.scope)?,
        slots,
        priority: value.priority,
    })
}

pub(super) fn claim(value: wire::Need) -> Result<AuthoredLimiterClaim> {
    Ok(AuthoredLimiterClaim {
        name: value.limiter.name,
        scope: scope(&value.limiter.scope)?,
        amount_millis: scaled_positive_millis(value.slots, "limiter slots")?,
        hold: hold(&value.hold)?,
    })
}

pub(super) fn limiter(value: wire::Limiter) -> Result<AuthoredLimiterDefinition> {
    Ok(match value {
        wire::Limiter::Lock { name, scope: raw } => AuthoredLimiterDefinition::Lock {
            name,
            scope: scope(&raw)?,
        },
        wire::Limiter::Resource {
            name,
            scope: raw,
            capacity,
            unit,
        } => {
            if unit.is_some() {
                bail!("v2 resource units are not active; use numeric slots")
            }
            AuthoredLimiterDefinition::Resource {
                name,
                scope: scope(&raw)?,
                capacity_millis: scaled_positive_millis(capacity, "resource capacity")?,
            }
        }
        wire::Limiter::RateLimit {
            name,
            scope: raw,
            burst,
            refill_per_second,
        } => AuthoredLimiterDefinition::RateLimit {
            name,
            scope: scope(&raw)?,
            burst: positive_u32(burst, "rate-limit burst")?,
            refill_millis_per_second: scaled_positive_millis(
                refill_per_second,
                "rate-limit refill_per_second",
            )?,
        },
        wire::Limiter::ProcessCap {
            name,
            scope: raw,
            max_running,
            match_pattern,
        } => {
            if match_pattern.is_some() {
                bail!("v2 process-cap matching is not active in this build")
            }
            AuthoredLimiterDefinition::ProcessCap {
                name,
                scope: scope(&raw)?,
                max_processes: positive_u32(max_running, "process cap")?,
            }
        }
    })
}

pub(super) fn queue(value: wire::QueueDefinition) -> Result<AuthoredQueueDefinition> {
    if value.max_pending.is_some() {
        bail!("v2 queue max_pending is not active in this build")
    }
    let discipline = match value.discipline.as_str() {
        "fifo" => QueueDiscipline::Fifo,
        "priority" => QueueDiscipline::Priority,
        other => bail!("unknown queue discipline `{other}`"),
    };
    Ok(AuthoredQueueDefinition {
        name: value.name,
        scope: scope(&value.scope)?,
        max_parallel_tasks: positive_u32(value.slots, "queue slots")?,
        discipline,
    })
}

pub(super) fn retry(value: wire::Retry) -> Result<RetryPolicy> {
    if !value.on_exit.is_empty() {
        bail!("v2 retry on_exit filtering is not active in this build")
    }
    let max_attempts = positive_u32(value.attempts, "retry attempts")?;
    let (backoff_millis, max_backoff_millis) = match value.backoff {
        wire::Backoff::Fixed { seconds } => {
            let delay = duration_millis(seconds, "fixed retry backoff")?;
            (delay, delay)
        }
        wire::Backoff::ExpJitter {
            min_s,
            max_s,
            jitter,
        } => {
            bail!(
                "v2 exponential jitter retry is not active in this build \
                 (min_s={min_s}, max_s={max_s}, jitter={jitter})"
            )
        }
    };
    Ok(RetryPolicy {
        max_attempts,
        backoff_millis,
        max_backoff_millis,
    })
}

pub(super) fn scope(value: &str) -> Result<DefinitionScope> {
    Ok(match value {
        "machine" => DefinitionScope::Node,
        "user" => DefinitionScope::Submitter,
        "project" => DefinitionScope::Project,
        "worktree" => DefinitionScope::Worktree,
        other => bail!("unknown scheduling scope `{other}`"),
    })
}

pub(super) fn scaled_positive_millis(value: f64, name: &str) -> Result<NonZeroU64> {
    NonZeroU64::new(exact_millis(value, name)?)
        .ok_or_else(|| anyhow::anyhow!("{name} must be positive"))
}

pub(super) fn duration_millis(value: f64, name: &str) -> Result<u64> {
    exact_millis(value, name)
}

fn exact_millis(value: f64, name: &str) -> Result<u64> {
    let scaled = value * 1_000.0;
    let rounded = scaled.round();
    let tolerance = f64::EPSILON * scaled.abs().max(1.0) * 8.0;
    if !value.is_finite()
        || value.is_sign_negative()
        || rounded >= u64::MAX as f64
        || (scaled - rounded).abs() > tolerance
    {
        bail!("{name} must be a finite non-negative value with millisecond precision")
    }
    Ok(rounded as u64)
}

fn positive_u32(value: u32, name: &str) -> Result<NonZeroU32> {
    NonZeroU32::new(value).ok_or_else(|| anyhow::anyhow!("{name} must be positive"))
}

fn hold(value: &str) -> Result<HoldMode> {
    Ok(match value {
        "during" => HoldMode::During,
        "at_start" => HoldMode::AtStart,
        other => bail!("unknown limiter hold mode `{other}`"),
    })
}
