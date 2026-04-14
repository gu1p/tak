use tak_core::crypto_provider::ensure_rustls_crypto_provider;

#[test]
fn ensure_rustls_crypto_provider_installs_a_default_provider() {
    ensure_rustls_crypto_provider();
    assert!(rustls::crypto::CryptoProvider::get_default().is_some());
}

#[test]
fn ensure_rustls_crypto_provider_is_idempotent() {
    ensure_rustls_crypto_provider();
    ensure_rustls_crypto_provider();
    assert!(rustls::crypto::CryptoProvider::get_default().is_some());
}
