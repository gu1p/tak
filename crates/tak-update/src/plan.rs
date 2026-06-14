//! The `run_update` use-case: resolve a target version, and (unless checking
//! only) download → verify → extract → install it.
//!
//! It depends only on the [`ReleaseClient`] and [`Installer`] ports, so it runs
//! end-to-end against fakes. The data flow enforces safety: bytes can only reach
//! [`Installer::install`] after [`verify_archive`](crate::verify::verify_archive)
//! produced a `VerifiedArchive` and
//! [`extract_binaries`](crate::archive::extract_binaries) turned it into
//! `Binaries`.

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::installer::{InstallReport, Installer};
use crate::release_client::ReleaseClient;
use crate::version::{Version, parse_version};

mod decision;
mod install;

use decision::{Decision, decide, resolve_tag};
use install::install;

/// Where each binary should be installed; `None` skips that binary.
#[derive(Debug, Clone, Default)]
pub struct Destinations {
    /// Destination for the `tak` CLI, if it should be (re)installed.
    pub tak: Option<PathBuf>,
    /// Destination for the `takd` daemon, if it should be (re)installed.
    pub takd: Option<PathBuf>,
}

/// Inputs controlling one update attempt.
pub struct UpdateOptions<'a> {
    /// `owner/name` of the release repository.
    pub repo: &'a str,
    /// Host target triple (e.g. `x86_64-unknown-linux-musl`).
    pub target: &'a str,
    /// The currently-running version.
    pub current: Version,
    /// Pin a specific tag instead of resolving the latest.
    pub requested_tag: Option<&'a str>,
    /// Permit installing an older version than `current`.
    pub allow_downgrade: bool,
    /// Resolve and report only; never download or install.
    pub check_only: bool,
    /// The trusted minisign public key (a `.pub` file's content).
    pub public_key: &'a str,
}

/// What `run_update` decided or did.
#[derive(Debug)]
pub enum UpdateAction {
    /// Already at (or newer than) the target; nothing to do.
    UpToDate,
    /// A newer version exists but `check_only` prevented installing it.
    Available,
    /// The new binaries were installed.
    Installed(InstallReport),
}

/// The result of an update attempt.
#[derive(Debug)]
pub struct UpdateOutcome {
    /// The version that was running.
    pub from: Version,
    /// The resolved target version.
    pub to: Version,
    /// The resolved target tag (e.g. `v0.1.7`).
    pub tag: String,
    /// The decision/result.
    pub action: UpdateAction,
}

/// Resolve a target version and, unless `check_only`, install it.
///
/// ```no_run
/// # // Reason: needs constructed `ReleaseClient`/`Installer` ports and performs network/filesystem IO.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub fn run_update<C, I>(
    client: &C,
    installer: &I,
    destinations: &Destinations,
    options: &UpdateOptions<'_>,
) -> Result<UpdateOutcome>
where
    C: ReleaseClient,
    I: Installer,
{
    let tag = resolve_tag(client, options)?;
    let to = parse_version(&tag).with_context(|| format!("parse release tag `{tag}`"))?;
    let action = match decide(options, to)? {
        Decision::UpToDate => UpdateAction::UpToDate,
        Decision::Available => UpdateAction::Available,
        Decision::Install => {
            UpdateAction::Installed(install(client, installer, destinations, options, &tag)?)
        }
    };
    Ok(UpdateOutcome {
        from: options.current,
        to,
        tag,
        action,
    })
}
