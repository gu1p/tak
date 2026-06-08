//! Production self-update entry point used by the `tak`/`takd` binaries and the
//! daemon loop: resolve the running binary and its sibling, then run the verified
//! [`run_update`] use-case with the real HTTPS client and filesystem installer.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::fs_installer::FsInstaller;
use crate::http::UreqReleaseClient;
use crate::install_target::{Updatability, resolve_running_binary, sibling_path, updatability};
use crate::plan::{Destinations, UpdateOptions, UpdateOutcome, run_update};
use crate::release_client::ReleaseClient;
use crate::target::host_target_triple;
use crate::version::parse_version;

/// Inputs for [`self_update`].
pub struct SelfUpdateRequest<'a> {
    /// The running binary's own name (`tak` or `takd`).
    pub primary_name: &'a str,
    /// The running binary's version string (e.g. `env!("TAK_VERSION")`).
    pub current_version: &'a str,
    /// Release repository (`owner/name`).
    pub repo: &'a str,
    /// Resolve/report only; never download or install.
    pub check_only: bool,
    /// Bypass the writable/system-path updatability guard (operator override).
    pub force: bool,
    /// Permit installing an older version than the running one.
    pub allow_downgrade: bool,
    /// Pin a specific tag instead of the latest.
    pub requested_tag: Option<&'a str>,
    /// Also update the co-located sibling binary if present.
    pub include_sibling: bool,
    /// The trusted minisign public key.
    pub public_key: &'a str,
}

/// Resolve destinations from the running binary and run the verified update
/// against the real GitHub release host.
pub fn self_update(request: &SelfUpdateRequest<'_>) -> Result<UpdateOutcome> {
    self_update_with(&UreqReleaseClient::new(), request)
}

/// Like [`self_update`], but with an injected release client (for integration tests).
pub fn self_update_with<C: ReleaseClient>(
    client: &C,
    request: &SelfUpdateRequest<'_>,
) -> Result<UpdateOutcome> {
    let exe = resolve_running_binary().context("resolve running binary path")?;
    let target = host_target_triple().context("detect host target triple")?;
    let current = parse_version(request.current_version)
        .with_context(|| format!("parse current version `{}`", request.current_version))?;

    if !request.check_only && !request.force {
        guard_updatable(&exe)?;
    }

    let destinations = resolve_destinations(&exe, request);
    let options = UpdateOptions {
        repo: request.repo,
        target: &target,
        current,
        requested_tag: request.requested_tag,
        allow_downgrade: request.allow_downgrade,
        check_only: request.check_only,
        public_key: request.public_key,
    };
    run_update(client, &FsInstaller, &destinations, &options)
}

fn guard_updatable(exe: &Path) -> Result<()> {
    match updatability(exe) {
        Updatability::Updatable => Ok(()),
        Updatability::NotWritable => {
            bail!(
                "cannot self-update: {} is not in a writable directory",
                exe.display()
            )
        }
        Updatability::SystemManaged => bail!(
            "cannot self-update: {} is under a system/package-managed path (use --force to override)",
            exe.display(),
        ),
    }
}

fn resolve_destinations(exe: &Path, request: &SelfUpdateRequest<'_>) -> Destinations {
    let sibling_name = if request.primary_name == "tak" {
        "takd"
    } else {
        "tak"
    };
    let mut destinations = Destinations::default();
    set_dest(&mut destinations, request.primary_name, exe.to_path_buf());
    if request.include_sibling {
        let sibling = sibling_path(exe, sibling_name);
        if sibling.exists() {
            set_dest(&mut destinations, sibling_name, sibling);
        }
    }
    destinations
}

fn set_dest(destinations: &mut Destinations, name: &str, path: PathBuf) {
    if name == "tak" {
        destinations.tak = Some(path);
    } else {
        destinations.takd = Some(path);
    }
}
