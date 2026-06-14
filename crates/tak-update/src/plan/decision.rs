//! Resolves the target release tag and decides whether to install it.

use std::cmp::Ordering;

use anyhow::{Context, Result, bail};

use crate::release_client::ReleaseClient;
use crate::version::Version;

use super::UpdateOptions;

pub(super) enum Decision {
    UpToDate,
    Available,
    Install,
}

pub(super) fn resolve_tag<C: ReleaseClient>(
    client: &C,
    options: &UpdateOptions<'_>,
) -> Result<String> {
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

pub(super) fn decide(options: &UpdateOptions<'_>, target: Version) -> Result<Decision> {
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

#[path = "decision_tests.rs"]
mod decision_tests;
