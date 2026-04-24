use anyhow::Result;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use tak_proto::TOR_INVITE_WORD_COUNT;

use super::app::RemoteAddApp;
use super::buffer_text::buffer_to_plain_text;
use super::frame;
use super::types::{Method, Screen};

const STYLE_TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);
const STYLE_ACTIVE: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD);
const STYLE_OK: Style = Style::new().fg(Color::Green).add_modifier(Modifier::BOLD);
const STYLE_ERROR: Style = Style::new().fg(Color::Red).add_modifier(Modifier::BOLD);
const STYLE_DIM: Style = Style::new().fg(Color::Gray).add_modifier(Modifier::DIM);

pub(super) fn render(frame: &mut ratatui::Frame<'_>, app: &RemoteAddApp) {
    match app.screen {
        Screen::Method => render_method(frame, app),
        Screen::Words => render_words(frame, app),
        Screen::Location => render_location(frame, app),
        Screen::Confirm => render_confirm(frame, app),
    }
}

pub(super) fn to_text(app: &RemoteAddApp) -> Result<String> {
    let backend = TestBackend::new(96, 32);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|frame| render(frame, app))?;
    Ok(buffer_to_plain_text(terminal.backend().buffer()))
}

fn render_method(frame: &mut ratatui::Frame<'_>, app: &RemoteAddApp) {
    let lines = vec![
        Line::from(vec![
            marker(app.selected_method == Method::Words),
            Span::styled("Words", method_style(app.selected_method == Method::Words)),
            Span::raw("  numbered Tor invite phrase"),
        ]),
        Line::from(vec![
            marker(app.selected_method == Method::Location),
            Span::styled(
                "Token or location",
                method_style(app.selected_method == Method::Location),
            ),
            Span::raw("  paste token, invite, or .onion host"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", STYLE_OK),
            Span::raw(" choose  "),
        ]),
    ];
    frame::render_lines(frame, " Add Remote ", lines, STYLE_TITLE);
}

fn render_words(frame: &mut ratatui::Frame<'_>, app: &RemoteAddApp) {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Tor invite words", STYLE_TITLE),
            Span::raw(format!(
                "  {}/{} filled",
                app.words.len(),
                TOR_INVITE_WORD_COUNT
            )),
        ]),
        Line::from(""),
    ];
    lines.extend(word_grid(app));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Input ", STYLE_DIM),
        Span::raw(if app.word_input.is_empty() {
            "_".to_string()
        } else {
            app.word_input.clone()
        }),
    ]));
    push_message(&mut lines, app);
    frame::render_lines(frame, " Words ", lines, STYLE_TITLE);
}

fn render_location(frame: &mut ratatui::Frame<'_>, app: &RemoteAddApp) {
    let mut lines = vec![
        Line::from(vec![Span::styled("Token or location", STYLE_TITLE)]),
        Line::from(""),
        Line::from("Paste a takd token, takd:tor invite, or Tor .onion host."),
        Line::from(""),
        Line::from(vec![
            Span::styled("Input ", STYLE_DIM),
            Span::raw(if app.location_input.is_empty() {
                "_".to_string()
            } else {
                app.location_input.clone()
            }),
        ]),
    ];
    push_message(&mut lines, app);
    frame::render_lines(frame, " Token or location ", lines, STYLE_TITLE);
}

fn render_confirm(frame: &mut ratatui::Frame<'_>, app: &RemoteAddApp) {
    let remote = app.remote.as_ref().expect("confirmation needs remote");
    let mut lines = vec![
        Line::from(vec![Span::styled("Confirm Remote", STYLE_TITLE)]),
        Line::from(""),
        field("Node", &remote.node_id),
        field("Display", &remote.display_name),
        field("Base URL", &remote.base_url),
        field("Transport", &remote.transport),
        field("Pools", &remote.pools.join(",")),
        field("Tags", &remote.tags.join(",")),
        field("Capabilities", &remote.capabilities.join(",")),
        Line::from(""),
        Line::from(vec![
            Span::styled("Enter", STYLE_OK),
            Span::raw(" save  "),
            Span::styled("Esc", STYLE_ERROR),
            Span::raw(" cancel"),
        ]),
    ];
    push_message(&mut lines, app);
    frame::render_lines(frame, " Confirm Remote ", lines, STYLE_TITLE);
}

fn word_grid(app: &RemoteAddApp) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    for row_start in (0..TOR_INVITE_WORD_COUNT).step_by(3) {
        let mut spans = Vec::new();
        for index in row_start..(row_start + 3).min(TOR_INVITE_WORD_COUNT) {
            let word = app
                .words
                .get(index)
                .cloned()
                .unwrap_or_else(|| "__________".to_string());
            let style = if app.words.get(index).is_some() {
                STYLE_OK
            } else {
                STYLE_DIM
            };
            spans.push(Span::styled(format!("{:02}", index + 1), STYLE_ACTIVE));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(format!("{word:<14}"), style));
            spans.push(Span::raw("  "));
        }
        lines.push(Line::from(spans));
    }
    lines
}

fn marker(active: bool) -> Span<'static> {
    if active {
        Span::styled("> ", STYLE_ACTIVE)
    } else {
        Span::raw("  ")
    }
}

fn method_style(active: bool) -> Style {
    if active { STYLE_ACTIVE } else { Style::new() }
}

fn field(label: &'static str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), STYLE_DIM),
        Span::raw(if value.is_empty() {
            "(none)".to_string()
        } else {
            value.to_string()
        }),
    ])
}

fn push_message(lines: &mut Vec<Line<'static>>, app: &RemoteAddApp) {
    if let Some(message) = &app.message {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            message.clone(),
            STYLE_ACTIVE,
        )]));
    }
}
