pub fn test_container_runtime() -> RuntimeSpec {
    RuntimeSpec {
        kind: Some(runtime_spec::Kind::Container(ContainerRuntime {
            image: Some("alpine:3.20".into()),
            dockerfile: None,
            build_context: None,
            resource_limits: Some(ContainerResourceLimits {
                cpu_cores: 1.0,
                memory_mb: 512,
            }),
        })),
    }
}
