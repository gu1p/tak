//! Pure containerized-runtime derivations: simulation policy, the metadata env
//! overrides, and the CPU-reservation parallelism cap.

use std::collections::BTreeMap;
use std::env;

use tak_core::model::ContainerResourceLimitsSpec;

pub(super) fn build_containerized_env_overrides(
    engine_name: &str,
    runtime_source: &str,
    image: &str,
    resource_limits: Option<&ContainerResourceLimitsSpec>,
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
    // Cap test-harness/data parallelism to the declared CPU reservation.
    // The container also gets a `nano_cpus` cgroup quota (see container
    // runtime), which makes Rust's cgroup-aware `available_parallelism()`
    // report ~cpu_cores; these env defaults are belt-and-suspenders for
    // the doctest harness (`RUST_TEST_THREADS`) and rayon, whose spikes
    // are the leading OOM trigger. They are defaults only: a step's own
    // env still overrides them (see `build_container_step_spec`). We do
    // NOT set `CARGO_BUILD_JOBS` here — tasks control it via a shell
    // `${CARGO_BUILD_JOBS:-N}` fallback that a container-env value would
    // otherwise override.
    if let Some(cpu_threads) = container_parallelism_cap(resource_limits) {
        env_overrides
            .entry("RUST_TEST_THREADS".to_string())
            .or_insert_with(|| cpu_threads.to_string());
        env_overrides
            .entry("RAYON_NUM_THREADS".to_string())
            .or_insert_with(|| cpu_threads.to_string());
    }
    env_overrides
}

/// Number of threads to which a containerized task's parallel work should be
/// capped, derived from the declared CPU reservation. Floors fractional cores
/// and never returns less than 1; `None` when no CPU reservation is declared.
///
/// ```rust
/// // Mirrors the derivation: floor fractional cores, but never below 1.
/// fn cap(cpu_cores: Option<f64>) -> Option<u64> {
///     let cpu_cores = cpu_cores?;
///     if !cpu_cores.is_finite() || cpu_cores <= 0.0 {
///         return None;
///     }
///     Some((cpu_cores.floor() as u64).max(1))
/// }
/// assert_eq!(cap(Some(2.7)), Some(2));
/// assert_eq!(cap(Some(0.5)), Some(1));
/// assert_eq!(cap(Some(0.0)), None);
/// assert_eq!(cap(None), None);
/// ```
fn container_parallelism_cap(resource_limits: Option<&ContainerResourceLimitsSpec>) -> Option<u64> {
    let cpu_cores = resource_limits?.cpu_cores?;
    if !cpu_cores.is_finite() || cpu_cores <= 0.0 {
        return None;
    }
    Some((cpu_cores.floor() as u64).max(1))
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
