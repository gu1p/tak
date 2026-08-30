use tak_core::v2::{DefinitionScope, HoldMode, LimiterDefinition};

use super::model::Lease;

pub(super) fn properties(
    definition: &LimiterDefinition,
) -> (&DefinitionScope, Option<&str>, u64, Lease) {
    match definition {
        LimiterDefinition::Lock {
            scope,
            scope_key,
            hold,
            ..
        } => (scope, scope_key.as_deref(), 1_000, hold_lease(*hold)),
        LimiterDefinition::RateLimit {
            scope,
            scope_key,
            permits,
            per_millis,
            ..
        } => (
            scope,
            scope_key.as_deref(),
            u64::from(permits.get()) * 1_000,
            Lease::Rate(per_millis.get()),
        ),
        LimiterDefinition::ProcessCap {
            scope,
            scope_key,
            max_processes,
            hold,
            ..
        } => (
            scope,
            scope_key.as_deref(),
            u64::from(max_processes.get()) * 1_000,
            hold_lease(*hold),
        ),
        LimiterDefinition::Resource {
            scope,
            scope_key,
            capacity_millis,
            hold,
            ..
        } => (
            scope,
            scope_key.as_deref(),
            capacity_millis.get(),
            hold_lease(*hold),
        ),
    }
}

pub(super) fn limiter_name(definition: &LimiterDefinition) -> &str {
    match definition {
        LimiterDefinition::Lock { name, .. }
        | LimiterDefinition::RateLimit { name, .. }
        | LimiterDefinition::ProcessCap { name, .. }
        | LimiterDefinition::Resource { name, .. } => name,
    }
}

fn hold_lease(hold: HoldMode) -> Lease {
    match hold {
        HoldMode::During => Lease::During,
        HoldMode::AtStart => Lease::AtStart,
    }
}
