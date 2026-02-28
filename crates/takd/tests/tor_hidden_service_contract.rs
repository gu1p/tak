//! Contract test for embedded Arti hidden-service wiring in `takd`.

#[test]
fn takd_embeds_arti_hidden_service_launch_path() {
    let source = include_str!("../src/lib.rs");
    assert!(
        source.contains("arti_client::TorClient"),
        "takd hidden-service mode must embed Arti in-process"
    );
    assert!(
        source.contains("launch_onion_service("),
        "takd hidden-service mode must launch onion service via Arti APIs"
    );
}
