//! `takd update` — update the installed `takd` (and co-located `tak`) from signed
//! releases. This is a separate short-lived process: it swaps the on-disk binaries
//! and tells the operator to restart the daemon service.

use std::path::Path;

use anyhow::Result;
use tak_update::plan::{UpdateAction, UpdateOutcome};
use tak_update::release_client::{DEFAULT_REPO, RELEASE_PUBLIC_KEY};
use tak_update::runner::{SelfUpdateRequest, self_update};
use takd::agent::read_config;

pub(super) fn run_update_command(
    config_root: &Path,
    check: bool,
    force: bool,
    version: Option<String>,
) -> Result<()> {
    // Config is optional: it only supplies a repo override and the sibling toggle.
    let config = read_config(config_root).ok();
    let repo = config
        .as_ref()
        .and_then(|config| config.auto_update.repo.clone());
    let include_sibling = config
        .as_ref()
        .map(|config| config.auto_update.include_sibling_tak)
        .unwrap_or(true);
    let outcome = self_update(&SelfUpdateRequest {
        primary_name: "takd",
        current_version: env!("TAKD_VERSION"),
        repo: repo.as_deref().unwrap_or(DEFAULT_REPO),
        check_only: check,
        force,
        allow_downgrade: force,
        requested_tag: version.as_deref(),
        include_sibling,
        public_key: RELEASE_PUBLIC_KEY,
    })?;
    report(&outcome);
    Ok(())
}

fn report(outcome: &UpdateOutcome) {
    match &outcome.action {
        UpdateAction::UpToDate => {
            println!("takd {} is up to date", outcome.from);
        }
        UpdateAction::Available => {
            println!(
                "update available: {} -> {} ({}); run `takd update` to install",
                outcome.from, outcome.to, outcome.tag,
            );
        }
        UpdateAction::Installed(report) => {
            println!(
                "updated {} -> {} ({}); installed: {}",
                outcome.from,
                outcome.to,
                outcome.tag,
                report.installed.join(", "),
            );
            println!(
                "restart the takd service to run the new daemon \
(e.g. `systemctl --user restart takd.service`)."
            );
            print_backups(&report.backups);
        }
    }
}

fn print_backups(backups: &[std::path::PathBuf]) {
    if backups.is_empty() {
        return;
    }
    let paths: Vec<String> = backups
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    println!("previous binaries saved at: {}", paths.join(", "));
    println!("delete these once satisfied, or restore one over its binary to roll back.");
}
