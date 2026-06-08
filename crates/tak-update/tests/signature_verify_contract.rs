use tak_update::verify::verify_signature;

const PUBLIC_KEY: &str = include_str!("data/test_release.pub");
const OTHER_PUBLIC_KEY: &str = include_str!("data/other_release.pub");
const SIGNATURE: &str = include_str!("data/test_archive.tar.gz.minisig");
const ARCHIVE: &[u8] = include_bytes!("data/test_archive.tar.gz");

#[test]
fn verifies_a_valid_signature() {
    assert!(verify_signature(ARCHIVE, SIGNATURE, PUBLIC_KEY).is_ok());
}

#[test]
fn accepts_bare_base64_public_key_line() {
    let bare = PUBLIC_KEY.lines().nth(1).unwrap();
    assert!(verify_signature(ARCHIVE, SIGNATURE, bare).is_ok());
}

#[test]
fn rejects_tampered_bytes() {
    let mut tampered = ARCHIVE.to_vec();
    tampered[0] ^= 0xff;
    assert!(verify_signature(&tampered, SIGNATURE, PUBLIC_KEY).is_err());
}

#[test]
fn rejects_wrong_public_key() {
    assert!(verify_signature(ARCHIVE, SIGNATURE, OTHER_PUBLIC_KEY).is_err());
}

#[test]
fn rejects_malformed_inputs() {
    assert!(verify_signature(ARCHIVE, "not a signature", PUBLIC_KEY).is_err());
    assert!(verify_signature(ARCHIVE, SIGNATURE, "not a key").is_err());
}
