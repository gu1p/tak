//! `tak update` — update the installed `tak`/`takd` binaries from signed releases.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use tak_update::legacy_drain::ensure_legacy_attempts_drained;
use tak_update::plan::{UpdateAction, UpdateOutcome};
use tak_update::release_client::{DEFAULT_REPO, RELEASE_PUBLIC_KEY};
use tak_update::runner::{SelfUpdateRequest, self_update};

pub(super) struct UpdateArgs {
    pub state_root: Option<PathBuf>,
    pub check: bool,
    pub force: bool,
    pub version: Option<String>,
}

pub(super) fn run_update_command(args: UpdateArgs) -> Result<()> {
    if !args.check {
        let state_root = match args.state_root {
            Some(state_root) => state_root,
            None => default_state_root()?,
        };
        ensure_legacy_attempts_drained(&state_root)?;
    }
    let outcome = self_update(&SelfUpdateRequest {
        primary_name: "tak",
        current_version: env!("TAK_VERSION"),
        repo: DEFAULT_REPO,
        check_only: args.check,
        force: args.force,
        allow_downgrade: args.force,
        requested_tag: args.version.as_deref(),
        include_sibling: true,
        public_key: RELEASE_PUBLIC_KEY,
    })?;
    report(&outcome);
    Ok(())
}

fn default_state_root() -> Result<PathBuf> {
    std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .map(|root| root.join("takd"))
        .map_err(|_| anyhow!("failed to resolve xdg_state_home"))
}

fn report(outcome: &UpdateOutcome) {
    match &outcome.action {
        UpdateAction::UpToDate => {
            println!("tak {} is up to date", outcome.from);
        }
        UpdateAction::Available => {
            println!(
                "update available: {} -> {} ({}); run `tak update` to install",
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
            if report.installed.iter().any(|name| name == "takd") {
                println!("restart the takd service to run the new daemon.");
            }
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
