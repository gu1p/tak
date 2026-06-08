use tak_update::verify::{parse_sha256_line, sha256_hex, verify_sha256};

#[test]
fn computes_known_empty_vector() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
}

#[test]
fn parses_shasum_line_and_lowercases() {
    let digest = "0".repeat(64);
    let line = format!("{digest}  tak-v0.1.7-x86_64-unknown-linux-musl.tar.gz");
    assert_eq!(parse_sha256_line(&line).unwrap(), digest);

    let upper = format!("{}  archive", "A".repeat(64));
    assert_eq!(parse_sha256_line(&upper).unwrap(), "a".repeat(64));
}

#[test]
fn rejects_malformed_checksum_lines() {
    assert!(parse_sha256_line("").is_err());
    assert!(parse_sha256_line("deadbeef  short.txt").is_err());
    assert!(parse_sha256_line(&format!("{}  x", "g".repeat(64))).is_err());
}

#[test]
fn verifies_and_rejects_tamper() {
    let bytes = b"release archive contents";
    let line = format!("{}  archive.tar.gz", sha256_hex(bytes));
    assert!(verify_sha256(bytes, &line).is_ok());
    assert!(verify_sha256(b"tampered contents", &line).is_err());

    let mut flipped: Vec<char> = sha256_hex(bytes).chars().collect();
    flipped[0] = if flipped[0] == '0' { '1' } else { '0' };
    let bad_line = format!(
        "{}  archive.tar.gz",
        flipped.into_iter().collect::<String>()
    );
    assert!(verify_sha256(bytes, &bad_line).is_err());
}
