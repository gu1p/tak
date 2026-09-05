//! Contract test for embedded Arti hidden-service wiring in `takd`.

#[test]
fn takd_embeds_arti_hidden_service_launch_path() {
    let source = include_str!("../src/service/tor/live_startup.rs");
    assert!(
        source.contains("arti_client::TorClient::create_bootstrapped(client_config).await"),
        "takd hidden-service mode must embed Arti in-process"
    );
    assert!(
        source.contains("tor_client.launch_onion_service(onion_config)"),
        "takd hidden-service mode must launch onion service via Arti APIs"
    );
}

#[test]
fn takd_enables_arti_client_side_onion_probe_feature() {
    let manifest = include_str!("../Cargo.toml");

    assert!(
        manifest.contains("\"onion-service-client\""),
        "takd self-probes its onion URL and must enable Arti's onion client feature"
    );
}
