use tak_core::model::ContainerResourceLimitsSpec;

use super::resource_admission::ResourceCapacity;
use super::resource_policy::RemoteResourcePolicy;

#[test]
fn omitted_limits_are_clamped_to_safe_node_capacity() {
    let policy = RemoteResourcePolicy::new(
        ResourceCapacity {
            cpu_cores: 2.0,
            memory_mb: 6144,
        },
        4.0,
        8192,
    );

    assert_eq!(
        policy.resolve(None),
        ContainerResourceLimitsSpec {
            cpu_cores: Some(2.0),
            memory_mb: Some(6144),
        }
    );
}

#[test]
fn explicitly_authored_limits_are_preserved_for_admission() {
    let policy = RemoteResourcePolicy::new(
        ResourceCapacity {
            cpu_cores: 2.0,
            memory_mb: 6144,
        },
        4.0,
        8192,
    );
    let authored = ContainerResourceLimitsSpec {
        cpu_cores: Some(8.0),
        memory_mb: Some(16384),
    };

    assert_eq!(policy.resolve(Some(authored.clone())), authored);
}
