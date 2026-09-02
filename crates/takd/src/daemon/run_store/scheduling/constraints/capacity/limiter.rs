use tak_core::v2::{DefinitionScope, HoldMode, LimiterDefinition};

use super::model::Lease;

pub(super) fn properties(
    definition: &LimiterDefinition,
) -> Option<(&DefinitionScope, Option<&str>, u64, Lease)> {
    match definition {
        LimiterDefinition::Lock {
            scope,
            scope_key,
            hold,
            ..
        } => Some((scope, scope_key.as_deref(), 1_000, hold_lease(*hold))),
        LimiterDefinition::RateLimit { .. } => None,
        LimiterDefinition::ProcessCap {
            scope,
            scope_key,
            max_processes,
            hold,
            ..
        } => Some((
            scope,
            scope_key.as_deref(),
            u64::from(max_processes.get()) * 1_000,
            hold_lease(*hold),
        )),
        LimiterDefinition::Resource {
            scope,
            scope_key,
            capacity_millis,
            hold,
            ..
        } => Some((
            scope,
            scope_key.as_deref(),
            capacity_millis.get(),
            hold_lease(*hold),
        )),
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

pub(super) fn process_match_pattern(definition: &LimiterDefinition) -> Option<&str> {
    match definition {
        LimiterDefinition::ProcessCap { match_pattern, .. } => match_pattern.as_deref(),
        _ => None,
    }
}

fn hold_lease(hold: HoldMode) -> Lease {
    match hold {
        HoldMode::During => Lease::During,
        HoldMode::AtStart => Lease::AtStart,
    }
}
