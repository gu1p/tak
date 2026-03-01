use super::*;

/// Resolves container engine deterministically: Docker first, then Podman on macOS only.
///
/// ```no_run
/// # // Reason: This behavior depends on host engine availability and is compile-checked only.
/// # use takd::daemon::transport::{ContainerEngine, ContainerEngineProbe, HostPlatform, select_container_engine_with_probe};
/// # struct Probe;
/// # impl ContainerEngineProbe for Probe {
/// #     fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String> {
/// #         match engine {
/// #             ContainerEngine::Docker => Ok(()),
/// #             ContainerEngine::Podman => Err("podman unavailable".to_string()),
/// #         }
/// #     }
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut probe = Probe;
/// let selected = select_container_engine_with_probe(HostPlatform::MacOs, &mut probe)?;
/// assert_eq!(selected, ContainerEngine::Docker);
/// # Ok(())
/// # }
/// ```
pub fn select_container_engine_with_probe(
    platform: HostPlatform,
    probe: &mut impl ContainerEngineProbe,
) -> Result<ContainerEngine> {
    if probe.probe(ContainerEngine::Docker).is_ok() {
        return Ok(ContainerEngine::Docker);
    }

    let mut attempted = vec![ContainerEngine::Docker.as_name()];
    if matches!(platform, HostPlatform::MacOs) {
        if probe.probe(ContainerEngine::Podman).is_ok() {
            return Ok(ContainerEngine::Podman);
        }
        attempted.push(ContainerEngine::Podman.as_name());
    }

    bail!(
        "no container engine available; attempted probes: {}",
        attempted.join(", ")
    );
}

/// Resolves container engine using the current host platform.
///
/// ```no_run
/// # // Reason: This behavior depends on host engine availability and is compile-checked only.
/// # use takd::daemon::transport::{ContainerEngine, ContainerEngineProbe, select_container_engine};
/// # struct Probe;
/// # impl ContainerEngineProbe for Probe {
/// #     fn probe(&mut self, engine: ContainerEngine) -> std::result::Result<(), String> {
/// #         match engine {
/// #             ContainerEngine::Docker => Ok(()),
/// #             ContainerEngine::Podman => Err("podman unavailable".to_string()),
/// #         }
/// #     }
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut probe = Probe;
/// let selected = select_container_engine(&mut probe)?;
/// assert_eq!(selected, ContainerEngine::Docker);
/// # Ok(())
/// # }
/// ```
pub fn select_container_engine(probe: &mut impl ContainerEngineProbe) -> Result<ContainerEngine> {
    select_container_engine_with_probe(HostPlatform::current(), probe)
}
