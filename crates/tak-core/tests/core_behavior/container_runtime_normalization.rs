use std::collections::BTreeMap;

use tak_core::model::{
    ContainerMountDef, ContainerResourceLimitsDef, ContainerRuntimeSourceInputSpec,
    RemoteRuntimeDef, validate_container_runtime_execution_spec,
};

#[test]
fn container_runtime_execution_spec_normalizes_valid_fields() {
    let runtime = RemoteRuntimeDef {
        kind: "containerized".to_string(),
        image: Some(
            " GHCR.IO/acme/api@SHA256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA "
                .to_string(),
        ),
        dockerfile: None,
        build_context: None,
        command: Some(vec![" /bin/sh ".to_string(), " -lc ".to_string(), "echo hi".to_string()]),
        mounts: vec![ContainerMountDef {
            source: " ./workspace ".to_string(),
            target: " /work/./src// ".to_string(),
            read_only: true,
        }],
        env: BTreeMap::from([
            ("APP_ENV".to_string(), "ci".to_string()),
            ("FEATURE_FLAG".to_string(), "1".to_string()),
        ]),
        resource_limits: Some(ContainerResourceLimitsDef {
            cpu_cores: Some(2.0),
            memory_mb: Some(512),
        }),
    };
    let spec = validate_container_runtime_execution_spec(&runtime).expect("valid runtime config");

    assert_eq!(
        spec.source,
        ContainerRuntimeSourceInputSpec::Image {
            image:
                "ghcr.io/acme/api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
        }
    );
    assert_eq!(spec.command, vec!["/bin/sh", "-lc", "echo hi"]);
    assert_eq!(spec.mounts[0].source, "./workspace");
    assert_eq!(spec.mounts[0].target, "/work/src");
    assert_eq!(spec.env.get("APP_ENV").map(String::as_str), Some("ci"));
    assert_eq!(
        spec.resource_limits.as_ref().and_then(|v| v.memory_mb),
        Some(512)
    );
}
