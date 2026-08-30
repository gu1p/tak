use std::num::{NonZeroU32, NonZeroU64};

use tak_core::v2::{
    DefinitionScope, HoldMode, LimiterClaim, LimiterDefinition, QueueDefinition, QueueDiscipline,
};

use crate::v2_resolved_run_support::sample_run;

#[test]
fn worktree_definitions_require_a_stable_scope_key() {
    let mut queue = sample_run();
    queue.queue_definitions = vec![QueueDefinition {
        name: "build".into(),
        scope: DefinitionScope::Worktree,
        scope_key: None,
        max_parallel_tasks: NonZeroU32::MIN,
        discipline: QueueDiscipline::Fifo,
    }];
    queue.jobs[0].queue = Some("build".into());
    assert!(queue.validate().is_err());
    queue.queue_definitions[0].scope_key = Some("worktree-a".into());
    assert!(queue.validate().is_ok());

    let mut limiter = sample_run();
    limiter.limiter_definitions = vec![LimiterDefinition::Lock {
        name: "exclusive".into(),
        scope: DefinitionScope::Worktree,
        scope_key: None,
        hold: HoldMode::During,
    }];
    limiter.jobs[0].limiter_claims = vec![LimiterClaim {
        name: "exclusive".into(),
        amount_millis: NonZeroU64::new(1_000).unwrap(),
    }];
    assert!(limiter.validate().is_err());
    let LimiterDefinition::Lock { scope_key, .. } = &mut limiter.limiter_definitions[0] else {
        unreachable!()
    };
    *scope_key = Some("worktree-a".into());
    assert!(limiter.validate().is_ok());
}
