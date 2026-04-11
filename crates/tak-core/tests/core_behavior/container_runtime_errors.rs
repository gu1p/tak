use std::collections::BTreeMap;

use tak_core::model::{
    ContainerMountDef, ContainerResourceLimitsDef, RemoteRuntimeDef,
    validate_container_runtime_execution_spec,
};

#[test]
fn container_runtime_execution_spec_rejects_invalid_mounts_and_limits() {
    let invalid_mount = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some("tak/test:v1".to_string()),
        dockerfile: None,
        build_context: None,
        command: None,
        mounts: vec![ContainerMountDef {
            source: "./workspace".to_string(),
            target: "work/src".to_string(),
            read_only: false,
        }],
        env: BTreeMap::new(),
        resource_limits: None,
    };
    assert!(validate_container_runtime_execution_spec(&invalid_mount).is_err());

    let invalid_limits = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some("tak/test:v1".to_string()),
        dockerfile: None,
        build_context: None,
        command: None,
        mounts: Vec::new(),
        env: BTreeMap::new(),
        resource_limits: Some(ContainerResourceLimitsDef {
            cpu_cores: Some(0.0),
            memory_mb: Some(0),
        }),
    };
    assert!(validate_container_runtime_execution_spec(&invalid_limits).is_err());
}

#[test]
fn container_runtime_execution_spec_redacts_sensitive_env_values_in_errors() {
    let runtime = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some("tak/test:v1".to_string()),
        dockerfile: None,
        build_context: None,
        command: None,
        mounts: Vec::new(),
        env: BTreeMap::from([(
            "SERVICE_TOKEN".to_string(),
            "super-secret-token\0".to_string(),
        )]),
        resource_limits: None,
    };
    let message = validate_container_runtime_execution_spec(&runtime)
        .expect_err("invalid env value should fail")
        .to_string();
    assert!(message.contains("SERVICE_TOKEN"));
    assert!(message.contains("<redacted>"));
    assert!(!message.contains("super-secret-token"));
}
