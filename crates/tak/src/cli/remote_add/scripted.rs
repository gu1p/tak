use anyhow::{Result, bail};

use super::app::RemoteAddApp;
use super::render::to_text;
use super::types::AddAction;
use super::{CommandOutcome, handle_command};

pub(super) async fn run(mut app: RemoteAddApp, script: &str) -> Result<()> {
    let mut transcript = vec![to_text(&app)?];
    for action in parse(script)? {
        let command = app.handle(action)?;
        match handle_command(&mut app, command).await? {
            CommandOutcome::Continue => transcript.push(to_text(&app)?),
            CommandOutcome::Saved(remote) => {
                transcript.push(to_text(&app)?);
                transcript.push(format!("added remote {}", remote.node_id));
                println!("{}", transcript.join("\n---\n"));
                return Ok(());
            }
            CommandOutcome::Cancelled => {
                transcript.push("remote add cancelled".to_string());
                println!("{}", transcript.join("\n---\n"));
                return Ok(());
            }
        }
    }
    bail!("remote add script ended before exit")
}

fn parse(script: &str) -> Result<Vec<AddAction>> {
    script
        .split(',')
        .map(str::trim)
        .filter(|step| !step.is_empty())
        .map(parse_step)
        .collect()
}

fn parse_step(step: &str) -> Result<AddAction> {
    match step {
        "up" => Ok(AddAction::Up),
        "down" => Ok(AddAction::Down),
        "enter" => Ok(AddAction::Enter),
        "esc" => Ok(AddAction::Back),
        "quit" | "q" => Ok(AddAction::Quit),
        "backspace" => Ok(AddAction::Backspace),
        "ctrl_u" => Ok(AddAction::ClearInput),
        "undo" => Ok(AddAction::UndoWord),
        _ => {
            if let Some(value) = step.strip_prefix("word:") {
                return Ok(AddAction::Word(value.to_string()));
            }
            if let Some(value) = step.strip_prefix("paste:") {
                return Ok(AddAction::Paste(value.to_string()));
            }
            bail!("unsupported remote add script step `{step}`");
        }
    }
}
