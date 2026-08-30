use std::num::{NonZeroU32, NonZeroU64};

use tak_core::v2::{
    DefinitionScope, HoldMode, LimiterClaim, LimiterDefinition, QueueDefinition, QueueDiscipline,
    RunSubmission,
};

pub fn project_queue(mut request: RunSubmission, max: u32) -> RunSubmission {
    request.run.queue_definitions = vec![QueueDefinition {
        name: "shared".into(),
        scope: DefinitionScope::Project,
        scope_key: None,
        max_parallel_tasks: NonZeroU32::new(max).unwrap(),
        discipline: QueueDiscipline::Fifo,
    }];
    request.run.jobs[0].queue = Some("shared".into());
    request
}

pub fn project_lock(request: RunSubmission, hold: HoldMode) -> RunSubmission {
    scoped_lock(request, DefinitionScope::Project, None, hold)
}

pub fn scoped_lock(
    mut request: RunSubmission,
    scope: DefinitionScope,
    scope_key: Option<&str>,
    hold: HoldMode,
) -> RunSubmission {
    request.run.limiter_definitions = vec![LimiterDefinition::Lock {
        name: "exclusive".into(),
        scope,
        scope_key: scope_key.map(str::to_owned),
        hold,
    }];
    request.run.jobs[0].limiter_claims = vec![LimiterClaim {
        name: "exclusive".into(),
        amount_millis: NonZeroU64::new(1_000).unwrap(),
    }];
    request
}

pub fn project_resource(mut request: RunSubmission, amount: u64) -> RunSubmission {
    request.run.limiter_definitions = vec![LimiterDefinition::Resource {
        name: "resource".into(),
        scope: DefinitionScope::Project,
        scope_key: None,
        capacity_millis: NonZeroU64::new(1_000).unwrap(),
        hold: HoldMode::During,
    }];
    request.run.jobs[0].limiter_claims = vec![LimiterClaim {
        name: "resource".into(),
        amount_millis: NonZeroU64::new(amount).unwrap(),
    }];
    request
}

pub fn project_process_cap(mut request: RunSubmission) -> RunSubmission {
    request.run.limiter_definitions = vec![LimiterDefinition::ProcessCap {
        name: "processes".into(),
        scope: DefinitionScope::Project,
        scope_key: None,
        max_processes: NonZeroU32::MIN,
        hold: HoldMode::During,
    }];
    claim(&mut request, "processes");
    request
}

pub fn project_rate_limit(mut request: RunSubmission) -> RunSubmission {
    request.run.limiter_definitions = vec![LimiterDefinition::RateLimit {
        name: "starts".into(),
        scope: DefinitionScope::Project,
        scope_key: None,
        burst: NonZeroU32::MIN,
        refill_millis_per_second: NonZeroU64::MIN,
    }];
    claim(&mut request, "starts");
    request
}

fn claim(request: &mut RunSubmission, name: &str) {
    request.run.jobs[0].limiter_claims = vec![LimiterClaim {
        name: name.into(),
        amount_millis: NonZeroU64::new(1_000).unwrap(),
    }];
}
