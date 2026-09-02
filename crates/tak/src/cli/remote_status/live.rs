use std::io::{Write, stdout};
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::cursor;
use crossterm::execute;
use crossterm::terminal::EnterAlternateScreen;
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};

#[path = "live_interrupt.rs"]
mod interrupt;
#[path = "live_terminal.rs"]
mod terminal_cleanup;

use super::fetch::fetch_snapshot;
use super::render::render_dashboard;
use super::view::RemoteStatusView;
use super::{RemoteStatusResult, fail_on_remote_errors};
use interrupt::{InterruptListener, PollOutcome, interruptible, wait_for_next_poll};
use terminal_cleanup::{TerminalCleanup, finish_terminal};

pub(super) async fn run_remote_status_dashboard(
    node_filters: &[String],
    watch: bool,
    poll_interval: Duration,
    max_polls: Option<usize>,
) -> Result<()> {
    let mut interrupt = watch.then(InterruptListener::new).transpose()?;
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
    let color_enabled = std::env::var_os("NO_COLOR").is_none();
    let mut polls = 0_usize;

    loop {
        let poll = fetch_dashboard_poll(
            node_filters,
            polls.saturating_add(1),
            watch,
            &mut terminal,
            color_enabled,
        );
        let snapshot = if let Some(interrupt) = interrupt.as_mut() {
            match interruptible(interrupt, poll).await? {
                PollOutcome::Completed(snapshot) => snapshot,
                PollOutcome::Interrupted => return finish_terminal(terminal, cleanup.take()),
            }
        } else {
            poll.await?
        };

        polls = polls.saturating_add(1);
        if !watch {
            if fail_on_remote_errors(&snapshot).is_err() {
                let _ = stdout().write_all(b"\n");
            }
            fail_on_remote_errors(&snapshot)?;
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
    node_filters: &[String],
    poll_index: usize,
    watch: bool,
    terminal: &mut Terminal<B>,
    color_enabled: bool,
) -> Result<Vec<RemoteStatusResult>>
where
    B::Error: std::error::Error + Send + Sync + 'static,
{
    let empty = RemoteStatusView::checking(&[], poll_index, watch);
    draw_dashboard(terminal, &empty, color_enabled)?;
    let snapshot = fetch_snapshot(node_filters).await?;
    let remotes = snapshot
        .iter()
        .map(|result| result.remote.clone())
        .collect::<Vec<_>>();
    let mut view = RemoteStatusView::checking(&remotes, poll_index, watch);
    for result in snapshot {
        view.mark_complete(result);
    }
    draw_dashboard(terminal, &view, color_enabled)?;
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
