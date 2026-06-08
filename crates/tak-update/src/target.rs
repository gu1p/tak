//! Release target-triple detection.
//!
//! Mirrors the `detect_target` logic in `get-tak.sh`: Linux builds are
//! `*-unknown-linux-musl`, macOS builds are `*-apple-darwin`, across `x86_64`
//! and `aarch64`. The pure [`target_triple`] takes the OS/arch as strings so it
//! is unit-testable; [`host_target_triple`] feeds it the running host's values.

/// Error returned when the host OS or CPU architecture has no published target.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UnsupportedTarget {
    /// The operating system has no published release target.
    #[error("unsupported operating system: {0}")]
    Os(String),
    /// The CPU architecture has no published release target.
    #[error("unsupported architecture: {0}")]
    Arch(String),
}

/// Map an OS and architecture name to a release target triple.
///
/// Accepts the spellings emitted by both `uname` and Rust's `std::env::consts`
/// (`Linux`/`linux`, `Darwin`/`macos`, `amd64`/`x86_64`, `arm64`/`aarch64`).
///
/// ```
/// use tak_update::target::target_triple;
/// assert_eq!(target_triple("linux", "x86_64").unwrap(), "x86_64-unknown-linux-musl");
/// assert_eq!(target_triple("macos", "arm64").unwrap(), "aarch64-apple-darwin");
/// assert!(target_triple("windows", "x86_64").is_err());
/// ```
pub fn target_triple(os: &str, arch: &str) -> Result<String, UnsupportedTarget> {
    let os_part = match os.trim().to_ascii_lowercase().as_str() {
        "linux" => "unknown-linux-musl",
        "darwin" | "macos" => "apple-darwin",
        other => return Err(UnsupportedTarget::Os(other.to_string())),
    };
    let arch_part = match arch.trim().to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        other => return Err(UnsupportedTarget::Arch(other.to_string())),
    };
    Ok(format!("{arch_part}-{os_part}"))
}

/// Resolve the running host's release target triple.
///
/// ```
/// // The suite only runs on supported hosts (linux/macOS, x86_64/aarch64).
/// assert!(tak_update::target::host_target_triple().is_ok());
/// ```
pub fn host_target_triple() -> Result<String, UnsupportedTarget> {
    target_triple(std::env::consts::OS, std::env::consts::ARCH)
}
