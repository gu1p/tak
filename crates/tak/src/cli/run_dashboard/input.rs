use std::future::pending;

use anyhow::Result;
use crossterm::event::{Event, EventStream};
use futures::{Stream, StreamExt};

use super::navigation::{NavigationAction, key_to_action};

pub(super) enum InputAction {
    Navigate(NavigationAction),
    Interrupt,
    Redraw,
    InputLost,
}

pub(super) enum StreamDisposition {
    Action(InputAction),
    Ignore,
    Disable,
}

pub(super) struct DashboardInput<Events = EventStream> {
    events: Option<Events>,
}

impl DashboardInput<EventStream> {
    pub(super) fn new(interactive: bool) -> Self {
        Self {
            events: interactive.then(EventStream::new),
        }
    }
}

impl<Events> DashboardInput<Events>
where
    Events: Stream<Item = std::io::Result<Event>> + Unpin,
{
    pub(super) async fn next(&mut self) -> Result<InputAction> {
        next_stream_action(&mut self.events).await
    }
}

pub(super) async fn next_stream_action<Events>(events: &mut Option<Events>) -> Result<InputAction>
where
    Events: Stream<Item = std::io::Result<Event>> + Unpin,
{
    loop {
        let item = if let Some(events) = events.as_mut() {
            events.next().await
        } else {
            return pending().await;
        };
        match classify_stream_item(item) {
            StreamDisposition::Action(action) => return Ok(action),
            StreamDisposition::Ignore => {}
            StreamDisposition::Disable => {
                *events = None;
                return Ok(InputAction::InputLost);
            }
        }
    }
}

pub(super) fn classify_stream_item(item: Option<std::io::Result<Event>>) -> StreamDisposition {
    let Some(Ok(event)) = item else {
        return StreamDisposition::Disable;
    };
    match event {
        Event::Key(key) => match key_to_action(key) {
            Some(NavigationAction::Interrupt) => StreamDisposition::Action(InputAction::Interrupt),
            Some(action) => StreamDisposition::Action(InputAction::Navigate(action)),
            None => StreamDisposition::Ignore,
        },
        Event::Resize(_, _) => StreamDisposition::Action(InputAction::Redraw),
        _ => StreamDisposition::Ignore,
    }
}
