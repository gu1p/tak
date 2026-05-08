use std::io::stdout;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use tokio::time::sleep;

use super::fetch::fetch_remote_status_result;
use super::render::render_dashboard;
use super::view::RemoteStatusView;
use super::{RemoteRecord, RemoteStatusResult};

pub(super) async fn run_remote_status_dashboard(
    remotes: &[RemoteRecord],
    watch: bool,
    poll_interval: Duration,
    max_polls: Option<usize>,
) -> Result<()> {
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend).context("create remote status terminal")?;
    terminal.clear().context("clear remote status terminal")?;
    let color_enabled = color_enabled();
    let mut polls = 0_usize;

    loop {
        let snapshot = fetch_dashboard_poll(
            remotes,
            polls.saturating_add(1),
            watch,
            &mut terminal,
            color_enabled,
        )
        .await?;

        polls = polls.saturating_add(1);
        if !watch {
            if snapshot.iter().any(|result| result.error.is_some()) {
                bail!("failed to query one or more remote nodes");
            }
            return Ok(());
        }
        if max_polls.is_some_and(|limit| polls >= limit) {
            return Ok(());
        }
        sleep(poll_interval).await;
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
