use tak_update::target::{host_target_triple, target_triple};

#[test]
fn maps_supported_targets() {
    assert_eq!(
        target_triple("linux", "x86_64").unwrap(),
        "x86_64-unknown-linux-musl",
    );
    assert_eq!(
        target_triple("Linux", "amd64").unwrap(),
        "x86_64-unknown-linux-musl",
    );
    assert_eq!(
        target_triple("linux", "aarch64").unwrap(),
        "aarch64-unknown-linux-musl",
    );
    assert_eq!(
        target_triple("macos", "arm64").unwrap(),
        "aarch64-apple-darwin",
    );
    assert_eq!(
        target_triple("Darwin", "x86_64").unwrap(),
        "x86_64-apple-darwin",
    );
}

#[test]
fn rejects_unsupported_os_and_arch() {
    assert!(target_triple("windows", "x86_64").is_err());
    assert!(target_triple("linux", "riscv64").is_err());
}

#[test]
fn host_target_is_supported() {
    assert!(host_target_triple().is_ok());
}
