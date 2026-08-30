use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, Transaction};
use tak_core::v2::{DefinitionScope, LimiterDefinition, ResolvedJob};

use super::limiter::limiter_name;
use super::model::{Context, Owner, owner};

mod state;
pub(super) use state::{BucketState, refill};

struct Plan {
    name: String,
    owner: Owner,
    burst: u32,
    refill: u64,
    state: BucketState,
}

pub(super) fn can_acquire(
    transaction: &Transaction<'_>,
    context: &Context<'_>,
    job: &ResolvedJob,
    node_id: &str,
) -> Result<bool> {
    Ok(project(transaction, context, job, node_id)?.is_some())
}

pub(super) fn acquire(
    transaction: &Transaction<'_>,
    context: &Context<'_>,
    job: &ResolvedJob,
    node_id: &str,
) -> Result<bool> {
    let Some(plans) = project(transaction, context, job, node_id)? else {
        return Ok(false);
    };
    for plan in plans {
        save(transaction, &plan)?;
    }
    Ok(true)
}

fn project(
    transaction: &Transaction<'_>,
    context: &Context<'_>,
    job: &ResolvedJob,
    node_id: &str,
) -> Result<Option<Vec<Plan>>> {
    let mut plans = Vec::new();
    for claim in &job.limiter_claims {
        let definition = context
            .run
            .limiter_definitions
            .iter()
            .find(|item| limiter_name(item) == claim.name)
            .ok_or_else(|| anyhow::anyhow!("unknown limiter `{}`", claim.name))?;
        let LimiterDefinition::RateLimit {
            name,
            scope,
            scope_key,
            burst,
            refill_millis_per_second,
        } = definition
        else {
            continue;
        };
        let owner = owner(context, scope, scope_key.as_deref(), node_id)?;
        let capacity = u64::from(burst.get()) * 1_000_000;
        let mut state = state(
            transaction,
            name,
            &owner,
            burst.get(),
            refill_millis_per_second.get(),
            capacity,
            context.now_ms,
        )?;
        let amount = claim
            .amount_millis
            .get()
            .checked_mul(1_000)
            .ok_or_else(|| anyhow::anyhow!("rate-limit claim overflow"))?;
        if amount > capacity {
            bail!("rate-limit claim `{name}` exceeds burst capacity")
        }
        if state.available_micros < amount {
            return Ok(None);
        }
        state.available_micros -= amount;
        plans.push(Plan {
            name: name.clone(),
            owner,
            burst: burst.get(),
            refill: refill_millis_per_second.get(),
            state,
        });
    }
    Ok(Some(plans))
}

fn state(
    transaction: &Transaction<'_>,
    name: &str,
    owner: &Owner,
    burst: u32,
    refill_rate: u64,
    capacity: u64,
    now_ms: u64,
) -> Result<BucketState> {
    let stored = transaction
        .query_row(
            "SELECT burst, refill_millis_per_second, available_micros, refilled_at_ms \
             FROM scheduler_rate_buckets WHERE limiter_name=?1 AND scope=?2 \
             AND owner_identity=?3 AND scope_key_present=?4 AND scope_key=?5",
            key_params(name, owner),
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((old_burst, old_rate, available, refilled_at)) = stored else {
        return Ok(BucketState::full(capacity, now_ms));
    };
    let old_burst = u32::try_from(old_burst)?;
    let old_rate = u64::try_from(old_rate)?;
    let stored = BucketState::new(u64::try_from(available)?, u64::try_from(refilled_at)?);
    if old_burst != burst || old_rate != refill_rate {
        return Ok(BucketState::full(capacity, now_ms));
    }
    refill(stored, capacity, refill_rate, now_ms)
}

fn save(transaction: &Transaction<'_>, plan: &Plan) -> Result<()> {
    let mut values = key_values(&plan.name, &plan.owner);
    values.extend([
        rusqlite::types::Value::Integer(i64::from(plan.burst)),
        rusqlite::types::Value::Integer(i64::try_from(plan.refill)?),
        rusqlite::types::Value::Integer(i64::try_from(plan.state.available_micros)?),
        rusqlite::types::Value::Integer(i64::try_from(plan.state.refilled_at_ms)?),
    ]);
    transaction.execute(
        "INSERT INTO scheduler_rate_buckets (limiter_name,scope,owner_identity,scope_key_present,scope_key,burst,refill_millis_per_second,available_micros,refilled_at_ms) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(limiter_name,scope,owner_identity,scope_key_present,scope_key) DO UPDATE SET \
         burst=excluded.burst, refill_millis_per_second=excluded.refill_millis_per_second, available_micros=excluded.available_micros, refilled_at_ms=excluded.refilled_at_ms",
        rusqlite::params_from_iter(values),
    )?;
    Ok(())
}

fn key_params(name: &str, owner: &Owner) -> impl rusqlite::Params {
    rusqlite::params_from_iter(key_values(name, owner))
}

fn key_values(name: &str, owner: &Owner) -> Vec<rusqlite::types::Value> {
    vec![
        name.to_owned().into(),
        scope_name(&owner.scope).to_owned().into(),
        owner.identity.clone().into(),
        i64::from(owner.scope_key.is_some()).into(),
        owner.scope_key.clone().unwrap_or_default().into(),
    ]
}

fn scope_name(scope: &DefinitionScope) -> &'static str {
    match scope {
        DefinitionScope::Run => "run",
        DefinitionScope::Submitter => "submitter",
        DefinitionScope::Project => "project",
        DefinitionScope::Worktree => "worktree",
        DefinitionScope::Node => "node",
    }
}
