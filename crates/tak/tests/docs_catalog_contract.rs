mod support;

use anyhow::{Result, anyhow};

use support::examples_catalog::load_catalog;

const HERO_EXAMPLES: [&str; 8] = [
    "small/01_hello_single_task",
    "small/04_cmd_with_env_and_cwd",
    "small/08_retry_fixed_fail_once",
    "medium/11_machine_lock_shared_ui",
    "medium/18_multi_package_monorepo",
    "large/24_full_feature_matrix_end_to_end",
    "large/25_remote_direct_build_and_artifact_roundtrip",
    "large/28_hybrid_local_remote_test_suite_failure_with_logs",
];

#[test]
fn hero_examples_expose_agent_authoring_metadata() -> Result<()> {
    let catalog = load_catalog()?;

    for name in HERO_EXAMPLES {
        let entry = catalog
            .example
            .iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| anyhow!("missing catalog entry `{name}`"))?;

        assert!(
            !entry.capabilities.is_empty(),
            "{name} missing capabilities metadata"
        );
        assert!(
            !entry.use_when.trim().is_empty(),
            "{name} missing use_when metadata"
        );
        assert!(
            !entry.project_shapes.is_empty(),
            "{name} missing project_shapes metadata"
        );
    }

    Ok(())
}
