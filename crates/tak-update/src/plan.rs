//! The `run_update` use-case: resolve a target version, and (unless checking
//! only) download → verify → extract → install it.
//!
//! It depends only on the [`ReleaseClient`] and [`Installer`] ports, so it runs
//! end-to-end against fakes. The data flow enforces safety: bytes can only reach
//! [`Installer::install`] after [`verify_archive`] produced a `VerifiedArchive`
//! and [`extract_binaries`] turned it into `Binaries`.

use std::cmp::Ordering;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::archive::extract_binaries;
use crate::installer::{BinaryArtifact, InstallPlan, InstallReport, Installer};
use crate::release_client::{ReleaseClient, ReleaseCoordinates};
use crate::verify::verify_archive;
use crate::version::{Version, parse_version};

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

enum Decision {
    UpToDate,
    Available,
    Install,
}

fn resolve_tag<C: ReleaseClient>(client: &C, options: &UpdateOptions<'_>) -> Result<String> {
    match options.requested_tag {
        Some(tag) => Ok(normalize_tag(tag)),
        None => client
            .resolve_latest_tag(options.repo)
            .context("resolve latest release tag"),
    }
}

fn normalize_tag(tag: &str) -> String {
    if tag.starts_with('v') {
        tag.to_string()
    } else {
        format!("v{tag}")
    }
}

fn decide(options: &UpdateOptions<'_>, target: Version) -> Result<Decision> {
    let decision = match target.cmp(&options.current) {
        Ordering::Equal => Decision::UpToDate,
        Ordering::Less if options.allow_downgrade => install_or_check(options),
        Ordering::Less if options.requested_tag.is_some() => bail!(
            "refusing downgrade to {target} (current {}); pass --force to allow",
            options.current
        ),
        Ordering::Less => Decision::UpToDate,
        Ordering::Greater => install_or_check(options),
    };
    Ok(decision)
}

fn install_or_check(options: &UpdateOptions<'_>) -> Decision {
    if options.check_only {
        Decision::Available
    } else {
        Decision::Install
    }
}

fn install<C, I>(
    client: &C,
    installer: &I,
    destinations: &Destinations,
    options: &UpdateOptions<'_>,
    tag: &str,
) -> Result<InstallReport>
where
    C: ReleaseClient,
    I: Installer,
{
    if destinations.tak.is_none() && destinations.takd.is_none() {
        bail!("no install destinations provided");
    }
    let coordinates = ReleaseCoordinates::new(options.repo, tag, options.target);
    let archive = client
        .download_archive(&coordinates)
        .context("download release archive")?;
    let sha256 = client
        .download_sha256(&coordinates)
        .context("download release checksum")?;
    let signature = client
        .download_signature(&coordinates)
        .context("download release signature")?;
    let verified = verify_archive(archive, &signature, &sha256, options.public_key)
        .context("verify release archive")?;
    let binaries = extract_binaries(&verified).context("extract release binaries")?;
    let plan = build_plan(tag, destinations, binaries);
    installer.install(&plan).context("install release binaries")
}

// Precondition (checked by the caller before any download): at least one of
// `destinations.tak` / `destinations.takd` is `Some`.
fn build_plan(
    tag: &str,
    destinations: &Destinations,
    binaries: crate::archive::Binaries,
) -> InstallPlan {
    let mut artifacts = Vec::new();
    if let Some(dest) = &destinations.tak {
        artifacts.push(BinaryArtifact::for_install(
            "tak",
            dest.clone(),
            binaries.tak,
        ));
    }
    if let Some(dest) = &destinations.takd {
        artifacts.push(BinaryArtifact::for_install(
            "takd",
            dest.clone(),
            binaries.takd,
        ));
    }
    InstallPlan::for_install(tag.to_string(), artifacts)
}
