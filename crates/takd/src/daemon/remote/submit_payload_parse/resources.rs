use super::*;
use tak_core::model::ContainerResourceLimitsSpec;

pub(super) fn parse_required_container_resource_limits(
    container: &tak_proto::ContainerRuntime,
) -> Result<ContainerResourceLimitsSpec> {
    let Some(limits) = container.resource_limits.as_ref() else {
        bail!("invalid_submit_fields: execution.runtime.container.resource_limits is required");
    };
    if !limits.cpu_cores.is_finite() || limits.cpu_cores <= 0.0 || limits.memory_mb == 0 {
        bail!("invalid_submit_fields: execution.runtime.container.resource_limits is invalid");
    }
    Ok(ContainerResourceLimitsSpec {
        cpu_cores: Some(limits.cpu_cores),
        memory_mb: Some(limits.memory_mb),
    })
}
