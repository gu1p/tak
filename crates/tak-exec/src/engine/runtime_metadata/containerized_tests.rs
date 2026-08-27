#![cfg(test)]
use tak_core::model::ContainerResourceLimitsSpec;

use super::build_containerized_env_overrides;

fn limits(cpu_cores: Option<f64>) -> ContainerResourceLimitsSpec {
    ContainerResourceLimitsSpec {
        cpu_cores,
        memory_mb: None,
    }
}

#[test]
fn env_overrides_include_all_base_runtime_keys() {
    let env = build_containerized_env_overrides("docker", "image", "img:tag", None);
    let expected = [
        ("TAK_RUNTIME", "containerized"),
        ("TAK_RUNTIME_ENGINE", "docker"),
        ("TAK_RUNTIME_SOURCE", "image"),
        ("TAK_CONTAINER_IMAGE", "img:tag"),
        ("TAK_REMOTE_RUNTIME", "containerized"),
        ("TAK_REMOTE_ENGINE", "docker"),
        ("TAK_REMOTE_CONTAINER_IMAGE", "img:tag"),
    ];
    for (key, value) in expected {
        assert_eq!(env.get(key).map(String::as_str), Some(value), "key {key}");
    }
}

#[test]
fn env_overrides_skip_parallelism_caps_without_cpu_reservation() {
    let env = build_containerized_env_overrides("podman", "dockerfile", "img", None);
    assert!(!env.contains_key("RUST_TEST_THREADS"));
    assert!(!env.contains_key("RAYON_NUM_THREADS"));
}

#[test]
fn cpu_reservations_do_not_become_parallelism_caps() {
    let env = build_containerized_env_overrides("docker", "image", "img", Some(&limits(Some(2.7))));
    assert!(!env.contains_key("RUST_TEST_THREADS"));
    assert!(!env.contains_key("RAYON_NUM_THREADS"));
}
