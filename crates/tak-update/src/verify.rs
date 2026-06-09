//! Authenticity (minisign signature) and integrity (SHA-256) verification of
//! downloaded release archives.
//!
//! [`verify_signature`] is the authenticity boundary: it checks an Ed25519
//! minisign signature against a public key embedded in the binary, so only
//! archives produced by the release pipeline's private key are accepted. The
//! [`verify_sha256`] check additionally defends against transport corruption /
//! partial downloads. Both run before an archive is extracted or installed; the
//! checksum alone proves integrity, not authenticity, since it ships from the same
//! release over the same channel.

use sha2::{Digest, Sha256};

/// Error returned by checksum parsing/verification.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChecksumError {
    /// The checksum line had no first token.
    #[error("checksum line is empty")]
    Empty,
    /// The digest was not 64 hex characters.
    #[error("checksum `{0}` is not 64 hex characters")]
    Malformed(String),
    /// The computed digest did not match the expected one.
    #[error("archive checksum mismatch")]
    Mismatch,
}

/// Lowercase hex SHA-256 of `bytes`.
///
/// ```rust
/// use tak_update::verify::sha256_hex;
/// assert_eq!(
///     sha256_hex(b""),
///     "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
/// );
/// ```
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Parse the digest out of a `shasum -a 256` line (`<hex>  <filename>`).
///
/// Only the first whitespace-delimited token is used; it is lowercased and must
/// be exactly 64 hex characters.
///
/// ```rust
/// use tak_update::verify::parse_sha256_line;
/// let digest = "a".repeat(64);
/// let line = format!("{digest}  tak-v0.1.7-x86_64-unknown-linux-musl.tar.gz");
/// assert_eq!(parse_sha256_line(&line).unwrap(), digest);
/// ```
pub fn parse_sha256_line(line: &str) -> Result<String, ChecksumError> {
    let token = line.split_whitespace().next().ok_or(ChecksumError::Empty)?;
    let lowered = token.to_ascii_lowercase();
    if lowered.len() != 64 || !lowered.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ChecksumError::Malformed(token.to_string()));
    }
    Ok(lowered)
}

/// Verify `bytes` against a `shasum -a 256` checksum line, comparing in constant time.
///
/// ```rust
/// use tak_update::verify::{sha256_hex, verify_sha256};
/// let bytes = b"release archive";
/// let line = format!("{}  archive.tar.gz", sha256_hex(bytes));
/// assert!(verify_sha256(bytes, &line).is_ok());
/// assert!(verify_sha256(b"tampered", &line).is_err());
/// ```
pub fn verify_sha256(bytes: &[u8], expected_line: &str) -> Result<(), ChecksumError> {
    let expected = parse_sha256_line(expected_line)?;
    let actual = sha256_hex(bytes);
    if constant_time_eq(actual.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(ChecksumError::Mismatch)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Error returned while verifying a release signature.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignatureError {
    /// The embedded/supplied public key could not be parsed.
    #[error("invalid minisign public key: {0}")]
    PublicKey(String),
    /// The `.minisig` signature file could not be parsed.
    #[error("invalid minisign signature: {0}")]
    Signature(String),
    /// The signature did not verify against the public key.
    #[error("release signature did not verify")]
    Invalid,
}

/// Verify a minisign signature (`minisig`, the `.minisig` file content) over
/// `bytes`, using `public_key` (a full `.pub` file's content or its bare base64
/// key line). Only modern (prehashed) minisign signatures are accepted.
///
/// ```no_run
/// # // Reason: needs a real minisign public key and matching `.minisig` signature.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn verify_signature(
    bytes: &[u8],
    minisig: &str,
    public_key: &str,
) -> Result<(), SignatureError> {
    let public_key = parse_public_key(public_key)?;
    let signature = minisign_verify::Signature::decode(minisig)
        .map_err(|err| SignatureError::Signature(err.to_string()))?;
    public_key
        .verify(bytes, &signature, false)
        .map_err(|_| SignatureError::Invalid)
}

fn parse_public_key(public_key: &str) -> Result<minisign_verify::PublicKey, SignatureError> {
    let trimmed = public_key.trim();
    let parsed = if trimmed.lines().count() >= 2 {
        minisign_verify::PublicKey::decode(trimmed)
    } else {
        minisign_verify::PublicKey::from_base64(trimmed)
    };
    parsed.map_err(|err| SignatureError::PublicKey(err.to_string()))
}

/// Error returned while verifying a downloaded release archive.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    /// The minisign signature did not verify (authenticity failure).
    #[error(transparent)]
    Signature(#[from] SignatureError),
    /// The SHA-256 checksum did not match (integrity failure).
    #[error(transparent)]
    Checksum(#[from] ChecksumError),
}

/// A release archive whose signature **and** checksum have both been verified.
///
/// The only public constructor is [`verify_archive`], so holding a
/// `VerifiedArchive` is proof the bytes are authentic. Extraction
/// ([`crate::archive::extract_binaries`]) requires one, which makes "download
/// then execute/install" impossible without first passing verification.
#[derive(Debug, Clone)]
pub struct VerifiedArchive(Vec<u8>);

impl VerifiedArchive {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Test-only constructor that bypasses verification. Compiled only under the
    /// `test-support` feature so production callers cannot reach it.
    ///
    /// ```no_run
    /// # // Reason: gated behind the `test-support` feature, off in default doctest builds.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "test-support")]
    pub fn for_test(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

/// Verify a downloaded archive's minisign signature and SHA-256, in that order.
///
/// The signature (authenticity) is checked first, then the checksum (integrity);
/// only then are the bytes wrapped as a [`VerifiedArchive`].
///
/// ```no_run
/// # // Reason: needs a real minisign key, signature, and checksum line.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn verify_archive(
    bytes: Vec<u8>,
    minisig: &str,
    sha256_line: &str,
    public_key: &str,
) -> Result<VerifiedArchive, VerifyError> {
    verify_signature(&bytes, minisig, public_key)?;
    verify_sha256(&bytes, sha256_line)?;
    Ok(VerifiedArchive(bytes))
}
