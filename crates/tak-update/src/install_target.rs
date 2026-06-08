//! Resolving the running binary and deciding whether it is safe to self-update.
//!
//! Self-update refuses package-manager-owned system paths (so it never fights
//! `apt`/`brew`/Nix) and read-only directories. The user-local install dir used by
//! `get-tak.sh`/`get-takd.sh` (`~/.local/bin`) is updatable. `current_exe` reads
//! `/proc/self/exe` on Linux, so a symlinked launcher already resolves to the real
//! file — and a symlink into a read-only store correctly trips these guards.

use std::path::{Path, PathBuf};

/// Whether the current install can be safely self-updated in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Updatability {
    /// The binary sits in a writable, non-system directory — safe to swap.
    Updatable,
    /// The binary's directory is not writable (read-only or owned by another user).
    NotWritable,
    /// The binary lives under a package-manager-owned system path.
    SystemManaged,
}

/// System path prefixes we refuse to self-update (package-manager territory).
const SYSTEM_PREFIXES: &[&str] = &[
    "/usr/",
    "/bin/",
    "/sbin/",
    "/opt/",
    "/nix/store/",
    "/Library/",
    "/System/",
];

/// The path of the running executable, with symlinks resolved.
///
/// `current_exe` already reads `/proc/self/exe` (canonical) on Linux; the extra
/// `canonicalize` makes macOS and any symlinked launcher resolve to the real file
/// before the system-path guards run, so a symlink into a read-only store is
/// correctly classified as [`Updatability::SystemManaged`]/[`Updatability::NotWritable`].
pub fn resolve_running_binary() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    Ok(std::fs::canonicalize(&exe).unwrap_or(exe))
}

/// The sibling binary path in the same directory (e.g. `tak` next to `takd`).
///
/// ```
/// use std::path::Path;
/// use tak_update::install_target::sibling_path;
/// assert_eq!(
///     sibling_path(Path::new("/home/u/.local/bin/takd"), "tak"),
///     Path::new("/home/u/.local/bin/tak"),
/// );
/// ```
pub fn sibling_path(primary: &Path, sibling_name: &str) -> PathBuf {
    match primary.parent() {
        Some(dir) => dir.join(sibling_name),
        None => PathBuf::from(sibling_name),
    }
}

/// Whether `path` is under a package-manager-owned system prefix.
///
/// ```
/// use std::path::Path;
/// use tak_update::install_target::is_system_managed_path;
/// assert!(is_system_managed_path(Path::new("/usr/local/bin/takd")));
/// assert!(!is_system_managed_path(Path::new("/home/u/.local/bin/takd")));
/// ```
pub fn is_system_managed_path(path: &Path) -> bool {
    let text = path.to_string_lossy();
    SYSTEM_PREFIXES
        .iter()
        .any(|prefix| text.starts_with(prefix))
}

/// Classify whether `target` can be self-updated in place.
pub fn updatability(target: &Path) -> Updatability {
    if is_system_managed_path(target) {
        return Updatability::SystemManaged;
    }
    match target.parent() {
        Some(dir) if dir_is_writable(dir) => Updatability::Updatable,
        _ => Updatability::NotWritable,
    }
}

fn dir_is_writable(dir: &Path) -> bool {
    tempfile::Builder::new()
        .prefix(".tak-update-wtest-")
        .tempfile_in(dir)
        .is_ok()
}
