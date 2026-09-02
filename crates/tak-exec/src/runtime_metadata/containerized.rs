//! Pure containerized-runtime derivations: simulation policy and metadata env
//! overrides.

use std::collections::BTreeMap;
use std::env;

use tak_core::model::ContainerResourceLimitsSpec;

pub(super) fn build_containerized_env_overrides(
    engine_name: &str,
    runtime_source: &str,
    image: &str,
    _resource_reservation: Option<&ContainerResourceLimitsSpec>,
) -> BTreeMap<String, String> {
    let mut env_overrides = BTreeMap::new();
    env_overrides.insert("TAK_RUNTIME".to_string(), "containerized".to_string());
    env_overrides.insert("TAK_RUNTIME_ENGINE".to_string(), engine_name.to_string());
    env_overrides.insert("TAK_RUNTIME_SOURCE".to_string(), runtime_source.to_string());
    env_overrides.insert("TAK_CONTAINER_IMAGE".to_string(), image.to_string());
    env_overrides.insert(
        "TAK_REMOTE_RUNTIME".to_string(),
        "containerized".to_string(),
    );
    env_overrides.insert("TAK_REMOTE_ENGINE".to_string(), engine_name.to_string());
    env_overrides.insert("TAK_REMOTE_CONTAINER_IMAGE".to_string(), image.to_string());
    env_overrides
}

pub(super) fn should_use_simulated_container_runtime() -> bool {
    // MOCK_CONTAINER simulates container execution so a takd node can run
    // inside a container with no nested Docker/Podman: skip the engine probe
    // and run steps directly on the host (container_plan stays None below).
    tak_core::mock::mock_container_enabled()
        || env::var("TAK_TEST_HOST_PLATFORM").is_ok()
        || env::var("TAK_TEST_CONTAINER_LIFECYCLE_FAILURES").is_ok()
}

#[path = "containerized_tests.rs"]
mod containerized_tests;
