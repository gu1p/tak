//! Contract test for remote runtime-service startup on the live Tor path.

#[test]
fn live_tor_starts_the_shared_remote_runtime_services() {
    let live = include_str!("../src/service/tor/live.rs");
    let services = include_str!("../src/daemon/remote/runtime_services.rs");
    assert!(
        live.contains("spawn_remote_runtime_services"),
        "the live Tor path must start the shared remote runtime services"
    );
    assert!(
        services.contains("spawn_remote_cleanup_janitor"),
        "the shared remote runtime services must include the cleanup janitor"
    );
}
