//! Shared switch for simulating container execution.
//!
//! When `MOCK_CONTAINER` is set to a truthy value, `tak`/`takd` skip every real
//! container-engine (Docker/Podman) interaction and instead simulate a
//! successful containerized run. This lets a `takd` remote node run *inside* a
//! container — where a nested container runtime is unavailable — so the Tor
//! transport can be exercised end to end without Docker-in-Docker.

/// Returns `true` when container operations should be simulated instead of
/// dispatched to a real Docker/Podman engine.
///
/// Controlled by the `MOCK_CONTAINER` environment variable. Recognised truthy
/// values (case-insensitive, surrounding whitespace ignored): `1`, `true`,
/// `yes`, `on`.
pub fn mock_container_enabled() -> bool {
    matches!(
        std::env::var("MOCK_CONTAINER")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref(),
        Some("1" | "true" | "yes" | "on")
    )
}
