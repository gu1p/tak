//! Background auto-update for the `takd` daemon: a jittered loop that checks for
//! newer signed releases and, when configured, drains in-flight work, swaps the
//! binary, and exits for the supervisor (systemd/launchd) to restart into the new
//! version. Startup reconciliation commits or rolls back a just-applied update.

use std::path::{Path, PathBuf};
use std::time::Duration;

mod drain;
mod loop_task;
mod state;

pub(crate) use loop_task::spawn_update_loop;

/// How long a freshly-restarted daemon must stay up before its update is
/// committed (backups removed).
const HEALTH_CONFIRM_DELAY: Duration = Duration::from_secs(120);

/// Reconcile any pending update at startup: roll back and exit on a crash-loop,
/// otherwise schedule a health-confirmation that removes the backups once the new
/// binary has proven it stays up.
///
/// ```no_run
/// # // Reason: reads update state from the filesystem, spawns a tokio task, and may call process::exit.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn reconcile_pending(state_root: &Path) {
    match state::reconcile_on_start(state_root) {
        state::BootDecision::None => {}
        state::BootDecision::Proceed => spawn_health_confirm(state_root.to_path_buf()),
        state::BootDecision::RolledBack => {
            tracing::error!(
                "auto-update: new binary failed to stay healthy after {} restarts; \
                 rolled back to the previous binary; exiting to restart",
                state::MAX_BOOT_ATTEMPTS,
            );
            std::process::exit(1);
        }
    }
}

fn spawn_health_confirm(state_root: PathBuf) {
    tokio::spawn(async move {
        tokio::time::sleep(HEALTH_CONFIRM_DELAY).await;
        state::commit(&state_root);
        tracing::info!("auto-update: confirmed healthy; previous binary backups removed");
    });
}
