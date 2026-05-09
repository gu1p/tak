use std::io::stdout;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use crossterm::cursor;
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use tokio::time::sleep;

#[path = "live_interrupt.rs"]
mod interrupt;
#[path = "live_terminal.rs"]
mod terminal_cleanup;

use super::fetch::fetch_remote_status_result;
use super::render::render_dashboard;
use super::view::RemoteStatusView;
use super::{RemoteRecord, RemoteStatusResult};
use interrupt::{InterruptListener, PollOutcome, interruptible, wait_for_next_poll};
use terminal_cleanup::{TerminalCleanup, finish_terminal};

pub(super) async fn run_remote_status_dashboard(
    remotes: &[RemoteRecord],
    watch: bool,
    poll_interval: Duration,
    max_polls: Option<usize>,
) -> Result<()> {
    let mut interrupt = if watch {
        Some(InterruptListener::new()?)
    } else {
        None
    };
    let mut out = stdout();
    let mut cleanup = if watch {
        execute!(out, EnterAlternateScreen, cursor::Hide)?;
        Some(TerminalCleanup::new())
    } else {
        None
    };
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend).context("create remote status terminal")?;
    terminal.clear().context("clear remote status terminal")?;
    let color_enabled = color_enabled();
    let mut polls = 0_usize;

    loop {
        let snapshot = if let Some(interrupt) = interrupt.as_mut() {
            match interruptible(
                interrupt,
                fetch_dashboard_poll(
                    remotes,
                    polls.saturating_add(1),
                    watch,
                    &mut terminal,
                    color_enabled,
                ),
            )
            .await?
            {
                PollOutcome::Completed(snapshot) => snapshot,
                PollOutcome::Interrupted => return finish_terminal(terminal, cleanup.take()),
            }
        } else {
            fetch_dashboard_poll(
                remotes,
                polls.saturating_add(1),
                watch,
                &mut terminal,
                color_enabled,
            )
            .await?
        };

        polls = polls.saturating_add(1);
        if !watch {
            if snapshot.iter().any(|result| result.error.is_some()) {
                bail!("failed to query one or more remote nodes");
            }
            return Ok(());
        }
        if max_polls.is_some_and(|limit| polls >= limit) {
            return finish_terminal(terminal, cleanup.take());
        }
        if wait_for_next_poll(
            interrupt
                .as_mut()
                .expect("watch mode installs an interrupt listener"),
            poll_interval,
        )
        .await?
        {
            return finish_terminal(terminal, cleanup.take());
        }
    }
}

async fn fetch_dashboard_poll<B: Backend>(
    remotes: &[RemoteRecord],
    poll_index: usize,
    watch: bool,
    terminal: &mut Terminal<B>,
    color_enabled: bool,
) -> Result<Vec<RemoteStatusResult>>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let mut view = RemoteStatusView::checking(remotes, poll_index, watch);
    draw_dashboard(terminal, &view, color_enabled)?;

    let mut pending = FuturesUnordered::new();
    for remote in remotes.iter().cloned() {
        pending.push(fetch_remote_status_result(remote));
    }

    while !pending.is_empty() {
        tokio::select! {
            maybe_result = pending.next() => {
                if let Some(result) = maybe_result {
                    view.mark_complete(result);
                    draw_dashboard(terminal, &view, color_enabled)?;
                }
            }
            _ = sleep(Duration::from_millis(120)) => {
                view.advance();
                draw_dashboard(terminal, &view, color_enabled)?;
            }
        }
    }

    Ok(view.completed_results())
}

fn draw_dashboard<B: Backend>(
    terminal: &mut Terminal<B>,
    view: &RemoteStatusView,
    color_enabled: bool,
) -> Result<()>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    terminal
        .draw(|frame| render_dashboard(frame, view, color_enabled))
        .context("draw remote status dashboard")?;
    Ok(())
}

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

#[cfg(test)]
#[path = "live_tests.rs"]
mod live_tests;
