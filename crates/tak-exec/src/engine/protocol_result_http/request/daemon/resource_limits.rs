use crate::engine::StrictRemoteTarget;

pub(super) fn runtime_resource_limits(target: &StrictRemoteTarget) -> (Option<f64>, Option<u64>) {
    let Some(tak_core::model::RemoteRuntimeSpec::Containerized {
        resource_limits: Some(limits),
        ..
    }) = target.runtime.as_ref()
    else {
        return (None, None);
    };
    (limits.cpu_cores, limits.memory_mb)
}
