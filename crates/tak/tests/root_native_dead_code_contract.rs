use std::collections::BTreeSet;

use anyhow::Result;
use tak_core::model::{Hold, Scope};

use crate::support::root_task_contracts::{load_root_spec, parse};

const NATIVE_DEAD_CODE_SCRIPT: &str = include_str!("../../../scripts/native_dead_code.sh");

#[test]
fn repo_root_native_dead_code_uses_install_dependency_and_cargo_lock() -> Result<()> {
    let spec = load_root_spec()?;
    let install = spec
        .tasks
        .get(&parse("//:native-dead-code-install"))
        .expect("native-dead-code-install task");
    let analyze = spec
        .tasks
        .get(&parse("//:native-dead-code"))
        .expect("native-dead-code task");

    for task in [install, analyze] {
        let need = task.needs.first().expect("shared Cargo lock need");
        assert_eq!(task.needs.len(), 1);
        assert_eq!(need.limiter.name, "cargo-check-workspace");
        assert_eq!(need.limiter.scope, Scope::Worktree);
        assert_eq!(need.slots, 1.0);
        assert!(matches!(need.hold, Hold::During));
    }

    assert_eq!(
        analyze.deps.iter().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from([parse("//:native-dead-code-install")]),
    );

    Ok(())
}

#[test]
fn native_dead_code_exposes_the_pinned_rustc_driver_library() {
    assert!(
        NATIVE_DEAD_CODE_SCRIPT.contains("rustc +\"${RUST_TOOLCHAIN}\" --print sysroot"),
        "native dead-code check must resolve the pinned toolchain sysroot",
    );
    assert!(NATIVE_DEAD_CODE_SCRIPT.contains("DYLD_LIBRARY_PATH"));
    assert!(NATIVE_DEAD_CODE_SCRIPT.contains("LD_LIBRARY_PATH"));
    assert!(NATIVE_DEAD_CODE_SCRIPT.contains("CARGO_BUILD_RUSTC_WRAPPER=\"\""));
}
