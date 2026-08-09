//! Contract test for remote cleanup startup on the live Tor readiness path.

#[test]
fn live_tor_readiness_starts_remote_cleanup_janitor() {
    let source = include_str!("../src/service/tor/live_readiness.rs");
    assert!(
        source.contains("spawn_remote_cleanup_janitor"),
        "the live Tor readiness path must start the remote cleanup janitor"
    );
}
