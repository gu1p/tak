use anyhow::{Result, bail};
use tak_proto::local_daemon::v2::{RunEvent, RunEventKind, RunLifecycleState};

use super::MISMATCH_DIAGNOSTIC;

pub(crate) struct EventPage<'a> {
    pub expected_run: &'a str,
    pub response_run: &'a str,
    pub after_event: u64,
    pub events: &'a [RunEvent],
    pub next_event: u64,
    pub state: RunLifecycleState,
    pub terminal: bool,
    pub logs_expired: bool,
}

pub(crate) fn validate_event_page(page: EventPage<'_>) -> Result<()> {
    let sequences_valid = page.events.iter().all(|event| event.seq > page.after_event)
        && page.events.windows(2).all(|pair| pair[0].seq < pair[1].seq)
        && page.next_event >= page.after_event
        && if page.logs_expired {
            page.events
                .last()
                .is_none_or(|event| event.seq <= page.next_event)
        } else {
            page.events.first().is_none_or(|event| {
                page.after_event
                    .checked_add(1)
                    .is_some_and(|expected| event.seq == expected)
            }) && page
                .events
                .last()
                .map_or(page.next_event == page.after_event, |event| {
                    event.seq == page.next_event
                })
        };
    let logs_are_safe = !page.logs_expired
        || page.events.iter().all(|event| {
            !matches!(event.kind, RunEventKind::Stdout | RunEventKind::Stderr)
                && event.chunk_base64.is_none()
        });
    if page.response_run != page.expected_run
        || !sequences_valid
        || !logs_are_safe
        || (page.terminal && !page.state.is_terminal())
    {
        bail!(MISMATCH_DIAGNOSTIC);
    }
    Ok(())
}
