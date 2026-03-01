//! Behavioral tests for Tor transport configuration validation.

use takd::daemon::transport::{
    ArtiSettings, TorTransportConfig, normalize_tor_transport_config, validate_tor_transport_config,
};

#[test]
fn accepts_valid_tor_transport_config() {
    let config = TorTransportConfig {
        onion_endpoint:
            "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion"
                .to_string(),
        service_auth_token: "service-token-123".to_string(),
        arti: ArtiSettings {
            socks5_addr: "127.0.0.1:9150".to_string(),
            data_dir: "/tmp/tak/arti".to_string(),
        },
    };

    validate_tor_transport_config(&config).expect("valid config should pass validation");
    let normalized =
        normalize_tor_transport_config(config).expect("valid config should normalize cleanly");
    assert_eq!(normalized.arti.socks5_addr, "127.0.0.1:9150");
}

#[test]
fn rejects_missing_required_tor_fields() {
    let config = TorTransportConfig {
        onion_endpoint: String::new(),
        service_auth_token: String::new(),
        arti: ArtiSettings {
            socks5_addr: String::new(),
            data_dir: String::new(),
        },
    };

    let error =
        validate_tor_transport_config(&config).expect_err("missing required fields must fail");
    let message = error.to_string();
    assert!(message.contains("onion endpoint is required"));
}

#[test]
fn rejects_malformed_onion_endpoint() {
    let config = TorTransportConfig {
        onion_endpoint: "http://example.com".to_string(),
        service_auth_token: "service-token-123".to_string(),
        arti: ArtiSettings {
            socks5_addr: "127.0.0.1:9150".to_string(),
            data_dir: "/tmp/tak/arti".to_string(),
        },
    };

    let error = validate_tor_transport_config(&config)
        .expect_err("non-onion endpoint must fail before transport creation");
    assert!(error.to_string().contains("must target a .onion host"));
}

#[test]
fn redacts_service_auth_token_from_validation_errors() {
    let secret = "secret token value should never appear";
    let config = TorTransportConfig {
        onion_endpoint:
            "http://abcdefghijklmnopqrstuvwxyz234567abcdefghijklmnopqrstuvwxyz2345.onion"
                .to_string(),
        service_auth_token: secret.to_string(),
        arti: ArtiSettings {
            socks5_addr: "127.0.0.1:9150".to_string(),
            data_dir: "/tmp/tak/arti".to_string(),
        },
    };

    let error = validate_tor_transport_config(&config)
        .expect_err("auth token containing spaces should fail");
    let message = error.to_string();
    assert!(message.contains("service auth token contains invalid characters"));
    assert!(!message.contains(secret));
}
