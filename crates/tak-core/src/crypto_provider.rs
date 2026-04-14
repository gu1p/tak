/// Installs a process-wide Rustls crypto provider when the process has not set one yet.
///
/// Tak uses Arti in both the CLI and daemon processes. Arti logs a warning when no Rustls
/// provider is configured up front, so we make that explicit during startup.
pub fn ensure_rustls_crypto_provider() {
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_rustls_crypto_provider;

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
}
