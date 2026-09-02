use anyhow::Result;
use tak_core::v2::{OutputSelector, SessionReuse};

use crate::support::root_task_contracts::{load_root_module, task};

const CARGO_JOBS: [&str; 4] = ["//:fmt-check", "//:lint", "//:test", "//:docs-check"];
const ISOLATED_JOBS: [&str; 6] = [
    "//:line-limits-check",
    "//:src-test-separation-check",
    "//:workflow-contract-check",
    "//:generated-artifact-ignore-check",
    "//:check-rust",
    "//:check",
];

#[test]
fn repo_root_scopes_the_cargo_paths_cache_to_cargo_jobs() -> Result<()> {
    let module = load_root_module()?;
    let cargo = task(&module, CARGO_JOBS[0])
        .session
        .as_ref()
        .expect("Cargo cache session");
    assert_eq!(cargo.name.as_deref(), Some("check-distributed"));
    assert_eq!(
        cargo.reuse,
        SessionReuse::Paths {
            paths: vec![
                OutputSelector::Path {
                    value: ".tmp/cargo-home".into(),
                },
                OutputSelector::Path {
                    value: ".tmp/cargo-target-local".into(),
                },
            ],
        }
    );

    let isolated = task(&module, ISOLATED_JOBS[0])
        .session
        .as_ref()
        .expect("isolated check session");
    assert_eq!(isolated.name.as_deref(), Some("check-isolated"));
    assert_eq!(isolated.reuse, SessionReuse::Workspace);
    assert_eq!(cargo.execution, isolated.execution);
    assert_eq!(cargo.context, isolated.context);

    for (labels, expected) in [(&CARGO_JOBS[..], cargo), (&ISOLATED_JOBS[..], isolated)] {
        for label in labels {
            let authored = task(&module, label);
            assert!(!authored.cascade_session, "{label} must not cascade");
            assert_eq!(authored.session.as_ref(), Some(expected), "{label} session");
        }
    }
    Ok(())
}
