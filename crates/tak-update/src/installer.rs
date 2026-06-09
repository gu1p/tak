//! The installer port: atomically place a validated set of release binaries.
//!
//! The update use-case decides *what* to install (which version, which binaries);
//! the [`Installer`] decides *how* to put bytes on disk safely. Tests use the real
//! [`crate::fs_installer::FsInstaller`] against a temp dir, or a fake.

use std::path::PathBuf;

/// One binary to install: its logical `name` (`tak`/`takd`), final `dest` path,
/// and the new `bytes` to write there.
///
/// Fields are crate-private so the only way to obtain one outside the crate is the
/// verified [`crate::plan::run_update`] path (or `for_test` under `test-support`).
/// This keeps unverified bytes from ever reaching [`Installer::install`].
#[derive(Debug, Clone)]
pub struct BinaryArtifact {
    pub(crate) name: String,
    pub(crate) dest: PathBuf,
    pub(crate) bytes: Vec<u8>,
}

impl BinaryArtifact {
    /// Crate-internal constructor, used by the verified [`crate::plan::run_update`] path.
    ///
    /// ```no_run
    /// # // Reason: crate-private constructor, not reachable from a doctest.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn for_install(name: impl Into<String>, dest: PathBuf, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            dest,
            bytes,
        }
    }

    /// Test-only constructor (compiled under the `test-support` feature).
    ///
    /// ```no_run
    /// # // Reason: gated behind the `test-support` feature, not built by a plain doctest.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "test-support")]
    pub fn for_test(name: impl Into<String>, dest: impl Into<PathBuf>, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            dest: dest.into(),
            bytes,
        }
    }
}

/// A set of binaries to install together for one release `tag`.
///
/// Like [`BinaryArtifact`], constructible outside the crate only via verified
/// extraction or the `test-support` `for_test` constructor.
#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub(crate) tag: String,
    pub(crate) artifacts: Vec<BinaryArtifact>,
}

impl InstallPlan {
    /// Crate-internal constructor, used by the verified [`crate::plan::run_update`] path.
    ///
    /// ```no_run
    /// # // Reason: crate-private constructor, not reachable from a doctest.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    pub(crate) fn for_install(tag: String, artifacts: Vec<BinaryArtifact>) -> Self {
        Self { tag, artifacts }
    }

    /// Test-only constructor (compiled under the `test-support` feature).
    ///
    /// ```no_run
    /// # // Reason: gated behind the `test-support` feature, not built by a plain doctest.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    #[cfg(feature = "test-support")]
    pub fn for_test(tag: impl Into<String>, artifacts: Vec<BinaryArtifact>) -> Self {
        Self {
            tag: tag.into(),
            artifacts,
        }
    }
}

/// Outcome of a successful install.
#[derive(Debug, Clone, Default)]
pub struct InstallReport {
    /// Names of the binaries that were swapped, in install order.
    pub installed: Vec<String>,
    /// `.bak` paths retained for rollback (empty entries for fresh installs).
    pub backups: Vec<PathBuf>,
}

/// Error returned while installing a [`InstallPlan`].
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// A candidate binary's `--version` did not match the planned tag.
    #[error("staged `{name}` reported `{got}`, expected `{want}`")]
    VersionMismatch {
        /// The binary that mismatched.
        name: String,
        /// The expected `--version` line.
        want: String,
        /// What the candidate actually printed.
        got: String,
    },
    /// The candidate binary could not be executed for validation.
    #[error("failed to validate staged `{0}`: {1}")]
    Probe(String, String),
    /// An on-disk swap operation failed (the live binaries were left intact or
    /// fully rolled back).
    #[error(transparent)]
    Swap(#[from] crate::swap::SwapError),
    /// A commit failed AND the rollback of already-installed binaries also failed,
    /// leaving the install dir in a mixed state that needs operator attention.
    #[error("install failed ({original}) and rollback also failed: {rollback}")]
    RollbackFailed {
        /// The original commit error that triggered the rollback.
        original: String,
        /// The rollback failures, joined.
        rollback: String,
    },
}

/// Atomically install a validated set of release binaries, all-or-nothing.
pub trait Installer {
    /// Validate every candidate, then swap them in, rolling back on partial failure.
    ///
    /// ```no_run
    /// # // Reason: trait method declaration; performs filesystem swaps and process probes.
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #     Ok(())
    /// # }
    /// ```
    fn install(&self, plan: &InstallPlan) -> Result<InstallReport, InstallError>;
}
