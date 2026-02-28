/// Resolves container engine deterministically: Docker first, then Podman on macOS only.
///
/// ```no_run
/// # // Reason: This behavior depends on host engine availability and is compile-checked only.
/// # use takd::{ContainerEngine, ContainerEngineProbe, HostPlatform, select_container_engine_with_probe};
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
/// # use takd::{ContainerEngine, ContainerEngineProbe, select_container_engine};
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

fn ensure_present(field: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("tor {field} is required");
    }
    Ok(())
}

fn is_valid_onion_endpoint(endpoint: &str) -> bool {
    let endpoint = endpoint.trim();
    let without_scheme = endpoint
        .strip_prefix("http://")
        .or_else(|| endpoint.strip_prefix("https://"))
        .unwrap_or(endpoint);
    let host_port = without_scheme.split('/').next().unwrap_or_default();
    let host = host_port.split(':').next().unwrap_or_default();
    host.ends_with(".onion")
}
