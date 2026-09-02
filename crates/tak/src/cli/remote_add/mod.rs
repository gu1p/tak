use anyhow::Result;

use super::remote_inventory::{RemoteRecord, add_remote, preview_remote};

mod app;
#[cfg(test)]
mod app_tests;
mod buffer_text;
mod frame;
mod location_input;
mod render;
mod scripted;
mod terminal;
#[cfg(test)]
mod terminal_tests;
mod types;

use app::RemoteAddApp;
use types::AppCommand;
pub(super) use types::StartMode;

pub(super) async fn run_remote_add(start: StartMode) -> Result<()> {
    let app = RemoteAddApp::new(start);
    if let Ok(script) = std::env::var("TAK_TEST_REMOTE_ADD_SCRIPT") {
        scripted::run(app, &script).await
    } else {
        terminal::run(app).await
    }
}

enum CommandOutcome {
    Continue,
    Saved(RemoteRecord),
    Cancelled,
}

async fn handle_command(app: &mut RemoteAddApp, command: AppCommand) -> Result<CommandOutcome> {
    match command {
        AppCommand::Continue => Ok(CommandOutcome::Continue),
        AppCommand::Cancel => Ok(CommandOutcome::Cancelled),
        AppCommand::Probe(token) => {
            match preview_remote(&token).await {
                Ok(remote) => app.show_remote(remote, token),
                Err(err) => app.show_error(format!("{err:#}")),
            }
            Ok(CommandOutcome::Continue)
        }
        AppCommand::Save(invite) => {
            let remote = add_remote(&invite).await?;
            Ok(CommandOutcome::Saved(remote))
        }
    }
}
