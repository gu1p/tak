use anyhow::{Result, bail};

use super::app::{AppAction, AppCommand, ScanApp};
use super::provider::CameraCatalog;
use super::render::to_text;
use crate::cli::remote_inventory::add_remote;

pub(super) async fn run(mut app: ScanApp, catalog: &dyn CameraCatalog, script: &str) -> Result<()> {
    let mut transcript = vec![to_text(&app)?];
    for step in parse(script)? {
        match app.handle(step, catalog)? {
            AppCommand::Continue => transcript.push(to_text(&app)?),
            AppCommand::Quit => {
                transcript.push("scan cancelled".to_string());
                println!("{}", transcript.join("\n---\n"));
                return Ok(());
            }
            AppCommand::AddToken(token) => {
                let remote = add_remote(&token).await?;
                transcript.push(to_text(&app)?);
                transcript.push(format!("added remote {}", remote.node_id));
                println!("{}", transcript.join("\n---\n"));
                return Ok(());
            }
        }
    }
    bail!("remote scan script ended before exit")
}

fn parse(script: &str) -> Result<Vec<AppAction>> {
    script
        .split(',')
        .map(str::trim)
        .filter(|step| !step.is_empty())
        .map(|step| match step {
            "up" => Ok(AppAction::Up),
            "down" => Ok(AppAction::Down),
            "enter" => Ok(AppAction::Enter),
            "esc" => Ok(AppAction::Back),
            "quit" | "q" => Ok(AppAction::Quit),
            "tick" => Ok(AppAction::Tick),
            _ => bail!("unsupported remote scan script step `{step}`"),
        })
        .collect()
}
