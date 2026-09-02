use anyhow::Result;
use tak_core::v2::{AuthoredLimiterDefinition, DefinitionScope, HoldMode};

use crate::support::root_task_contracts::{load_root_module, task};

const CARGO_CHECK_LOCK: &str = "cargo-check-workspace";

#[test]
fn repo_root_cargo_checks_allow_one_per_worker() -> Result<()> {
    let module = load_root_module()?;

    assert!(
        module.limiter_definitions.iter().any(|limiter| matches!(
            limiter,
            AuthoredLimiterDefinition::Lock { name, scope }
                if name == CARGO_CHECK_LOCK && *scope == DefinitionScope::Node
        )),
        "missing per-worker Cargo lock"
    );

    for label in ["//:fmt-check", "//:lint", "//:test", "//:docs-check"] {
        let task = task(&module, label);
        assert_eq!(
            task.limiter_claims.len(),
            1,
            "{label} should use one lock need"
        );

        let need = &task.limiter_claims[0];
        assert_eq!(need.name, CARGO_CHECK_LOCK, "{label} lock name");
        assert_eq!(need.scope, DefinitionScope::Node, "{label} lock scope");
        assert_eq!(need.amount_millis.get(), 1_000, "{label} lock slots");
        assert!(
            matches!(need.hold, HoldMode::During),
            "{label} should hold the lock for the full Cargo task"
        );
    }

    Ok(())
}
