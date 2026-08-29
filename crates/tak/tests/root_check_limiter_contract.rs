use anyhow::Result;
use tak_core::model::{Hold, LimiterDef, Scope};

use crate::support::root_task_contracts::{load_root_spec, parse};

const CARGO_CHECK_LOCK: &str = "cargo-check-workspace";

#[test]
fn repo_root_cargo_checks_share_one_worktree_lock() -> Result<()> {
    let spec = load_root_spec()?;

    assert!(
        spec.limiters.values().any(|limiter| matches!(
            limiter,
            LimiterDef::Lock { name, scope }
                if name == CARGO_CHECK_LOCK && *scope == Scope::Worktree
        )),
        "missing shared Cargo worktree lock"
    );

    for label in ["//:fmt-check", "//:lint", "//:test", "//:docs-check"] {
        let task = spec.tasks.get(&parse(label)).expect("cargo task");
        assert_eq!(task.needs.len(), 1, "{label} should use one lock need");

        let need = &task.needs[0];
        assert_eq!(need.limiter.name, CARGO_CHECK_LOCK, "{label} lock name");
        assert_eq!(need.limiter.scope, Scope::Worktree, "{label} lock scope");
        assert_eq!(need.slots, 1.0, "{label} lock slots");
        assert!(
            matches!(need.hold, Hold::During),
            "{label} should hold the lock for the full Cargo task"
        );
    }

    Ok(())
}
