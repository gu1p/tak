//! Contract test for remote cleanup startup on the Tor hidden-service runtime path.

#[test]
fn tor_hidden_service_runtime_starts_remote_cleanup_janitor() {
    let source = include_str!("../src/daemon/remote/tor_server.rs");
    assert!(
        source.contains("spawn_remote_cleanup_janitor(context.shared_status_state())"),
        "the real Tor hidden-service runtime must start the remote cleanup janitor"
    );
}
