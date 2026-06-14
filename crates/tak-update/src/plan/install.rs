//! The download → verify → extract → install pipeline for a resolved release.

use anyhow::{Context, Result, bail};

use crate::archive::extract_binaries;
use crate::installer::{BinaryArtifact, InstallPlan, InstallReport, Installer};
use crate::release_client::{ReleaseClient, ReleaseCoordinates};
use crate::verify::verify_archive;

use super::{Destinations, UpdateOptions};

pub(super) fn install<C, I>(
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
