fn is_sensitive_runtime_env_key(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("token")
        || lower.contains("secret")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("key")
}

fn normalize_runtime_resource_limits(
    resource_limits: Option<&ContainerResourceLimitsDef>,
) -> Result<Option<ContainerResourceLimitsSpec>, ContainerRuntimeExecutionSpecError> {
    let Some(resource_limits) = resource_limits else {
        return Ok(None);
    };

    if let Some(cpu_cores) = resource_limits.cpu_cores
        && (!cpu_cores.is_finite() || cpu_cores <= 0.0 || cpu_cores > 256.0)
    {
        return Err(ContainerRuntimeExecutionSpecError::InvalidCpuCores);
    }

    if let Some(memory_mb) = resource_limits.memory_mb
        && memory_mb == 0
    {
        return Err(ContainerRuntimeExecutionSpecError::InvalidMemoryMb);
    }

    if resource_limits.cpu_cores.is_none() && resource_limits.memory_mb.is_none() {
        return Ok(None);
    }

    Ok(Some(ContainerResourceLimitsSpec {
        cpu_cores: resource_limits.cpu_cores,
        memory_mb: resource_limits.memory_mb,
    }))
}
