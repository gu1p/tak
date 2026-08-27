use tak_core::model::ContainerResourceLimitsSpec;

use super::resource_admission::ResourceCapacity;
use super::resource_envelope::ResourceEnvelope;
use super::resource_policy::RemoteResourcePolicy;

#[test]
fn omitted_resources_remain_elastic() {
    let policy = test_policy();

    let authored = policy.resolve(None);
    assert_eq!(
        authored,
        ContainerResourceLimitsSpec {
            cpu_cores: None,
            memory_mb: None,
        }
    );
    let startup = policy.startup_claim(&authored);
    assert!((startup.cpu_cores - 2.0).abs() < f64::EPSILON);
    assert_eq!(startup.memory_mb, 6144);
}

#[test]
fn explicitly_authored_limits_are_preserved_for_admission() {
    let policy = test_policy();
    let authored = ContainerResourceLimitsSpec {
        cpu_cores: Some(8.0),
        memory_mb: Some(16384),
    };

    assert_eq!(policy.resolve(Some(authored.clone())), authored);
}

fn test_policy() -> RemoteResourcePolicy {
    let capacity = ResourceCapacity {
        cpu_cores: 2.0,
        memory_mb: 6144,
    };
    let zero = ResourceCapacity {
        cpu_cores: 0.0,
        memory_mb: 0,
    };
    RemoteResourcePolicy::with_envelope(
        ResourceEnvelope {
            total: capacity,
            margin: zero,
            host_reserve: zero,
            workload: capacity,
        },
        4.0,
        8192,
    )
}
