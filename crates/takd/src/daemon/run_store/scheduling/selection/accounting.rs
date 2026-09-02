use anyhow::Result;
use rusqlite::Transaction;

use super::Usage;
use crate::daemon::scheduler::SchedulerNode;

pub(super) fn reserved_usage(transaction: &Transaction<'_>, node: &SchedulerNode) -> Result<Usage> {
    let mut statement = transaction.prepare(
        "SELECT cpu_millis, memory_bytes, execution_slots, accepted_at_ms \
         FROM run_attempts WHERE node_id = ?1 AND released_at_ms IS NULL",
    )?;
    let rows = statement.query_map([&node.node_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<i64>>(3)?.is_some(),
        ))
    })?;
    let mut usage = Usage::default();
    for row in rows {
        let (cpu, memory, slots, accepted) = row?;
        let (cpu_total, memory_total, slot_total) = if accepted {
            (
                &mut usage.cpu_millis,
                &mut usage.memory_bytes,
                &mut usage.execution_slots,
            )
        } else {
            (
                &mut usage.unaccepted_cpu_millis,
                &mut usage.unaccepted_memory_bytes,
                &mut usage.unaccepted_execution_slots,
            )
        };
        add(cpu_total, cpu, "CPU reservation")?;
        add(memory_total, memory, "memory reservation")?;
        add(slot_total, slots, "slot reservation")?;
        usage.attempt_count = usage
            .attempt_count
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("reservation count overflow"))?;
    }
    Ok(usage)
}

fn add(total: &mut u64, value: i64, label: &str) -> Result<()> {
    *total = total
        .checked_add(u64::try_from(value)?)
        .ok_or_else(|| anyhow::anyhow!("{label} total overflow"))?;
    Ok(())
}
